//! textDocument/typeDefinition handler.
//!
//! Navigates from a symbol usage to its type definition.
//! For example, from `engine : Engine` to `part def Engine`.

use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{Location, Position, Range, Url};

impl LspServer {
    /// Get the type definition location for a symbol at the given position.
    ///
    /// This navigates from a usage to its type definition, e.g.:
    /// - `engine : Engine` → goes to `part def Engine`
    /// - `vehicle :> VehiclePart` → goes to `part def VehiclePart`
    pub fn get_type_definition(&mut self, uri: &Url, position: Position) -> Option<Location> {
        let path = uri_to_path(uri)?;
        let path_str = path.to_string_lossy();

        let analysis = self.analysis_host.analysis();

        // Get file ID for the HIR layer
        let file_id = analysis.get_file_id(&path_str)?;

        // Use the Analysis goto_type_definition method
        let result = analysis.goto_type_definition(file_id, position.line, position.character);

        // Get the first target (if any)
        let target = result.targets.into_iter().next()?;

        // Convert FileId back to path
        let def_path = analysis.get_file_path(target.file)?;
        let def_uri = Url::from_file_path(def_path).ok()?;

        Some(Location {
            uri: def_uri,
            range: Range {
                start: Position {
                    line: target.start_line,
                    character: target.start_col,
                },
                end: Position {
                    line: target.end_line,
                    character: target.end_col,
                },
            },
        })
    }
}
