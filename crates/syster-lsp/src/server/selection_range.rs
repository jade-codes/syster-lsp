//! Selection range support for the LSP server

use super::LspServer;
use async_lsp::lsp_types::{Position, Range, SelectionRange};
use std::path::Path;
use syster::ide;

impl LspServer {
    /// Get selection ranges at the given positions in a document
    ///
    /// Returns a vector of SelectionRange chains, one for each input position.
    pub fn get_selection_ranges(
        &mut self,
        file_path: &Path,
        positions: Vec<Position>,
    ) -> Vec<SelectionRange> {
        let path_str = file_path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        let Some(file_id) = analysis.get_file_id(&path_str) else {
            return positions
                .iter()
                .map(|p| Self::default_selection_range(*p))
                .collect();
        };

        // Collect ranges from analysis first
        let all_ranges: Vec<Vec<ide::SelectionRange>> = positions
            .iter()
            .map(|pos| analysis.selection_ranges(file_id, pos.line, pos.character))
            .collect();

        // Now build the results without borrowing self
        all_ranges
            .into_iter()
            .zip(positions.iter())
            .map(|(ranges, pos)| {
                if ranges.is_empty() {
                    Self::default_selection_range(*pos)
                } else {
                    Self::build_selection_range_chain(ranges)
                }
            })
            .collect()
    }

    /// Build a SelectionRange chain from IDE SelectionRanges (innermost to outermost)
    fn build_selection_range_chain(ranges: Vec<ide::SelectionRange>) -> SelectionRange {
        // ranges are ordered from smallest (innermost) to largest (outermost)
        // We need to build a chain where innermost points to outermost as parent
        let mut iter = ranges.into_iter().rev(); // Start from largest (outermost)

        let outermost = iter.next().expect("ranges should not be empty");
        let mut current = SelectionRange {
            range: Range {
                start: Position {
                    line: outermost.start_line,
                    character: outermost.start_col,
                },
                end: Position {
                    line: outermost.end_line,
                    character: outermost.end_col,
                },
            },
            parent: None,
        };

        // Build chain from outermost to innermost
        for r in iter {
            current = SelectionRange {
                range: Range {
                    start: Position {
                        line: r.start_line,
                        character: r.start_col,
                    },
                    end: Position {
                        line: r.end_line,
                        character: r.end_col,
                    },
                },
                parent: Some(Box::new(current)),
            };
        }

        current
    }

    /// Create a default selection range (single character) when no AST node is found
    fn default_selection_range(pos: Position) -> SelectionRange {
        SelectionRange {
            range: Range {
                start: pos,
                end: Position {
                    line: pos.line,
                    character: pos.character + 1,
                },
            },
            parent: None,
        }
    }
}
