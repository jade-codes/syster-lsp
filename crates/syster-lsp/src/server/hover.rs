use super::LspServer;
use super::helpers::{decode_uri_component, uri_to_path};
use async_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Range, Url};
use tracing::debug;

impl LspServer {
    /// Get hover information for a symbol at the given position
    ///
    /// Uses the new HIR-based IDE layer for hover content generation.
    pub fn get_hover(&mut self, uri: &Url, position: Position) -> Option<Hover> {
        debug!(
            "[HOVER] get_hover called for uri={}, position={}:{}",
            uri, position.line, position.character
        );

        let path = uri_to_path(uri)?;
        debug!("[HOVER] path={:?}", path);

        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        // Get file ID for the new HIR layer
        let file_id = analysis.get_file_id(&path_str)?;

        // Use the Analysis hover method
        let result = analysis.hover(file_id, position.line, position.character)?;

        debug!("[HOVER] Found symbol, building hover content");

        // Get the qualified name from the result to find references
        let mut contents = result.contents.clone();

        // Add "Referenced by:" section with clickable links
        if let Some(qualified_name) = result.qualified_name.as_ref() {
            contents =
                Self::add_references_section_from_analysis(&analysis, &contents, qualified_name);
        }

        // Convert to LSP Hover
        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(contents)),
            range: Some(Range {
                start: Position {
                    line: result.start_line,
                    character: result.start_col,
                },
                end: Position {
                    line: result.end_line,
                    character: result.end_col,
                },
            }),
        })
    }

    /// Add "Referenced by:" section with clickable file links.
    fn add_references_section_from_analysis(
        analysis: &syster::ide::Analysis<'_>,
        content: &str,
        qualified_name: &str,
    ) -> String {
        // Get the simple name from qualified name for matching type_refs
        // type_refs store simple names like "Base", not "Test::Base"
        let simple_name = qualified_name.rsplit("::").next().unwrap_or(qualified_name);

        // Collect references to this symbol
        // Match both the simple name and qualified name since type_refs may use either
        // type_refs are now TypeRefKind (Simple or Chain), so we flatten with as_refs()
        let mut references: Vec<_> = analysis
            .symbol_index()
            .all_symbols()
            .filter(|sym| {
                sym.type_refs
                    .iter()
                    .flat_map(|trk| trk.as_refs())
                    .any(|tr| {
                        tr.target.as_ref() == qualified_name || tr.target.as_ref() == simple_name
                    })
            })
            .flat_map(|sym| {
                sym.type_refs
                    .iter()
                    .flat_map(|trk| trk.as_refs())
                    .filter(|tr| {
                        tr.target.as_ref() == qualified_name || tr.target.as_ref() == simple_name
                    })
                    .map(move |tr| (sym.file, tr.start_line, tr.start_col))
            })
            .collect();

        if references.is_empty() {
            return content.to_string();
        }

        // Sort for deterministic output
        references.sort_by_key(|(file, line, col)| (*file, *line, *col));

        let mut result = content.to_string();
        let count = references.len();
        let plural = if count == 1 { "" } else { "s" };
        result.push_str(&format!("\n**Referenced by:** ({count} usage{plural})\n"));

        for (file_id, line, col) in references {
            if let Some(path) = analysis.get_file_path(file_id)
                && let Ok(uri) = Url::from_file_path(path)
            {
                let file_name = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let decoded_file_name = decode_uri_component(file_name);
                let display_line = line + 1; // 1-indexed for display
                let display_col = col + 1;
                result.push_str(&format!(
                    "- [{decoded_file_name}:{display_line}:{display_col}]({}#L{display_line})\n",
                    uri
                ));
            }
        }

        result
    }
}
