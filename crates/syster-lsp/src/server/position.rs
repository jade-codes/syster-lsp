use super::LspServer;
use async_lsp::lsp_types::{Position, Range};
use std::path::Path;

impl LspServer {
    /// Find the symbol at the given position using semantic information.
    ///
    /// Uses the new HIR-based SymbolIndex for lookup.
    pub fn find_symbol_at_position(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Option<(String, Range)> {
        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();
        let file_id = analysis.get_file_id(&path_str)?;

        // First check if cursor is on a type reference
        for sym in analysis.symbol_index().symbols_in_file(file_id) {
            for type_ref_kind in &sym.type_refs {
                // Check if position is within this type reference kind
                if let Some((_, type_ref)) =
                    type_ref_kind.part_at(position.line, position.character)
                {
                    let range = Range {
                        start: Position {
                            line: type_ref.start_line,
                            character: type_ref.start_col,
                        },
                        end: Position {
                            line: type_ref.end_line,
                            character: type_ref.end_col,
                        },
                    };
                    // Return the target of the type ref
                    return Some((type_ref.target.to_string(), range));
                }
            }
        }

        // Then check if cursor is on a symbol definition
        for sym in analysis.symbol_index().symbols_in_file(file_id) {
            if contains_position(
                sym.start_line,
                sym.start_col,
                sym.end_line,
                sym.end_col,
                position.line,
                position.character,
            ) {
                let range = Range {
                    start: Position {
                        line: sym.start_line,
                        character: sym.start_col,
                    },
                    end: Position {
                        line: sym.end_line,
                        character: sym.end_col,
                    },
                };
                return Some((sym.qualified_name.to_string(), range));
            }
        }

        None
    }
}

fn contains_position(
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    line: u32,
    col: u32,
) -> bool {
    let after_start = line > start_line || (line == start_line && col >= start_col);
    let before_end = line < end_line || (line == end_line && col <= end_col);
    after_start && before_end
}
