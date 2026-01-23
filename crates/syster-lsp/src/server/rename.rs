use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{Position, PrepareRenameResponse, Range, TextEdit, Url, WorkspaceEdit};
use std::collections::HashMap;

impl LspServer {
    /// Prepare rename: validate that the symbol at the position can be renamed
    /// Returns the range of the symbol and its current text, or None if rename is not valid
    pub fn prepare_rename(
        &mut self,
        uri: &Url,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        let path = uri_to_path(uri)?;
        let (element_name, range) = self.find_symbol_at_position(&path, position)?;

        let analysis = self.analysis_host.analysis();

        // Try qualified name lookup first, then simple name
        let symbol = analysis
            .symbol_index()
            .lookup_qualified(&element_name)
            .or_else(|| {
                // Try simple name lookup and find a definition
                analysis
                    .symbol_index()
                    .lookup_simple(&element_name)
                    .into_iter()
                    .find(|s| s.kind.is_definition())
            })?;

        // Get the simple name for display
        let simple_name = symbol.name.to_string();

        // Return the range where the rename will happen and the current text
        Some(PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: simple_name,
        })
    }

    /// Rename a symbol at the given position
    ///
    /// Finds all references to the symbol and generates a WorkspaceEdit
    /// to rename them all to the new name.
    pub fn get_rename_edits(
        &mut self,
        uri: &Url,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let path = uri_to_path(uri)?;
        let path_str = path.to_string_lossy();
        let (_element_name, _) = self.find_symbol_at_position(&path, position)?;

        let analysis = self.analysis_host.analysis();
        let file_id = analysis.get_file_id(&path_str)?;

        // Use find_references to get all locations (with include_declaration=true)
        let result = analysis.find_references(
            file_id,
            position.line,
            position.character,
            true, // include declaration
        );

        if result.is_empty() {
            return None;
        }

        // Convert to WorkspaceEdit
        let mut edits_by_file: HashMap<Url, Vec<TextEdit>> = HashMap::new();

        for reference in result.references {
            if let Some(ref_path) = analysis.get_file_path(reference.file)
                && let Ok(file_uri) = Url::from_file_path(ref_path)
            {
                let range = Range {
                    start: Position {
                        line: reference.start_line,
                        character: reference.start_col,
                    },
                    end: Position {
                        line: reference.end_line,
                        character: reference.end_col,
                    },
                };
                edits_by_file.entry(file_uri).or_default().push(TextEdit {
                    range,
                    new_text: new_name.to_string(),
                });
            }
        }

        Some(WorkspaceEdit {
            changes: Some(edits_by_file),
            document_changes: None,
            change_annotations: None,
        })
    }
}
