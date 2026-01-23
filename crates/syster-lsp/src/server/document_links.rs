use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{DocumentLink, Position, Range, Url};

impl LspServer {
    /// Get document links for imports and qualified references in the document
    ///
    /// Returns a list of clickable links that navigate to:
    /// 1. Import statements - links to the definition of the imported symbol
    /// 2. Type references - links to specialized types, typed definitions, etc.
    ///
    /// Uses the new HIR-based IDE layer.
    pub fn get_document_links(&mut self, uri: &Url) -> Vec<DocumentLink> {
        let path = match uri_to_path(uri) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        let file_id = match analysis.get_file_id(&path_str) {
            Some(id) => id,
            None => return Vec::new(),
        };

        // Use the Analysis document_links method
        let ide_links = analysis.document_links(file_id);

        // Convert to LSP DocumentLinks
        ide_links
            .into_iter()
            .filter_map(|link| {
                // Convert target FileId to URI
                let target_path = analysis.get_file_path(link.target_file)?;
                let target_uri = Url::from_file_path(target_path).ok()?;

                // Add line number to URI fragment for jump-to-line
                let target_line = link.target_line + 1; // 1-indexed
                let target_uri_with_line =
                    Url::parse(&format!("{}#L{}", target_uri, target_line)).ok()?;

                Some(DocumentLink {
                    range: Range {
                        start: Position {
                            line: link.start_line,
                            character: link.start_col,
                        },
                        end: Position {
                            line: link.end_line,
                            character: link.end_col,
                        },
                    },
                    target: Some(target_uri_with_line),
                    tooltip: Some(link.tooltip.to_string()),
                    data: None,
                })
            })
            .collect()
    }
}
