use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{Location, Position, Range, Url};

impl LspServer {
    /// Find all references to a symbol at the given position
    ///
    /// Uses the new HIR-based IDE layer for find-references.
    pub fn get_references(
        &mut self,
        uri: &Url,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let path = uri_to_path(uri)?;
        let path_str = path.to_string_lossy();

        let analysis = self.analysis_host.analysis();

        // Get file ID for the new HIR layer
        let file_id = analysis.get_file_id(&path_str)?;

        // Use the Analysis find_references method
        let result = analysis.find_references(
            file_id,
            position.line,
            position.character,
            include_declaration,
        );

        // Convert to LSP Locations
        let locations: Vec<Location> = result
            .references
            .into_iter()
            .filter_map(|reference| {
                let ref_path = analysis.get_file_path(reference.file)?;
                let ref_uri = Url::from_file_path(ref_path).ok()?;
                Some(Location {
                    uri: ref_uri,
                    range: Range {
                        start: Position {
                            line: reference.start_line,
                            character: reference.start_col,
                        },
                        end: Position {
                            line: reference.end_line,
                            character: reference.end_col,
                        },
                    },
                })
            })
            .collect();

        Some(locations)
    }
}
