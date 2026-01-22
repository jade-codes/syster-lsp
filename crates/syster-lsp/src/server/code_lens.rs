use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{CodeLens, Command, Location, Position, Range, Url};
use syster::hir::SymbolKind;

impl LspServer {
    /// Get code lenses for a document
    ///
    /// Shows inline commands above definitions:
    /// - "N references" - clickable to show all references
    pub fn get_code_lenses(&mut self, uri: &Url) -> Vec<CodeLens> {
        let Some(path) = uri_to_path(uri) else {
            return Vec::new();
        };
        
        let analysis = self.analysis_host.analysis();
        let path_str = path.to_string_lossy();
        
        let Some(file_id) = analysis.get_file_id(&path_str) else {
            return Vec::new();
        };

        let mut lenses = Vec::new();

        // Get symbols in this file from the SymbolIndex
        for symbol in analysis.symbol_index().symbols_in_file(file_id) {
            // Only show code lens for definitions
            if !symbol.kind.is_definition() && !matches!(symbol.kind, SymbolKind::Package) {
                continue;
            }

            let range = Range {
                start: Position {
                    line: symbol.start_line,
                    character: symbol.start_col,
                },
                end: Position {
                    line: symbol.end_line,
                    character: symbol.end_col,
                },
            };

            // Count references using type_refs
            let qualified_name = symbol.qualified_name.as_ref();
            let references = Self::collect_reference_locations_from_analysis(&analysis, qualified_name);
            let reference_count = references.len();

            // Only show code lens if there are references
            if reference_count > 0 {
                let uri_value = serde_json::Value::String(uri.to_string());
                let Ok(position_value) = serde_json::to_value(Position {
                    line: range.start.line,
                    character: range.start.character,
                }) else {
                    continue;
                };
                let Ok(locations_value) = serde_json::to_value(references) else {
                    continue;
                };

                let lens = CodeLens {
                    range,
                    command: Some(Command {
                        title: format!(
                            "{} reference{}",
                            reference_count,
                            if reference_count == 1 { "" } else { "s" }
                        ),
                        command: "syster.showReferences".to_string(),
                        arguments: Some(vec![uri_value, position_value, locations_value]),
                    }),
                    data: None,
                };
                lenses.push(lens);
            }
        }

        lenses
    }

    /// Collect all reference locations for a qualified name
    fn collect_reference_locations_from_analysis(analysis: &syster::ide::Analysis<'_>, qualified_name: &str) -> Vec<Location> {
        analysis.symbol_index()
            .all_symbols()
            .flat_map(|sym| {
                sym.type_refs
                    .iter()
                    .flat_map(|trk| trk.as_refs())
                    .filter(|tr| tr.target.as_ref() == qualified_name)
                    .filter_map(|tr| {
                        let path = analysis.get_file_path(sym.file)?;
                        let uri = Url::from_file_path(path).ok()?;
                        Some(Location {
                            uri,
                            range: Range {
                                start: Position {
                                    line: tr.start_line,
                                    character: tr.start_col,
                                },
                                end: Position {
                                    line: tr.end_line,
                                    character: tr.end_col,
                                },
                            },
                        })
                    })
            })
            .collect()
    }
}
