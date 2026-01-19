use super::LspServer;
use async_lsp::lsp_types::{Position, Range};
use std::path::PathBuf;

impl LspServer {
    /// Find the symbol at the given position using semantic information.
    ///
    /// This uses the ReferenceIndex and SymbolTable populated during semantic analysis,
    /// avoiding fragile text extraction. The approach is:
    /// 1. Check if cursor is on a reference (usage site) → ReferenceIndex
    /// 2. Check if cursor is on a definition → SymbolTable
    pub fn find_symbol_at_position(
        &self,
        path: &PathBuf,
        position: Position,
    ) -> Option<(String, Range)> {
        use super::helpers::span_to_lsp_range;

        let file_path_str = path.to_string_lossy().to_string();
        let pos = syster::core::Position::new(position.line as usize, position.character as usize);

        // 1. Check if cursor is on a reference (e.g., type annotation, specialization target)
        if let Some(target_name) = self
            .workspace
            .reference_index()
            .get_reference_at_position(&file_path_str, pos)
        {
            // Resolve the target to get full symbol info
            let resolver = self.resolver();
            if let Some(symbol) = resolver.resolve(target_name) {
                let range = symbol
                    .span()
                    .map(|s| span_to_lsp_range(&s))
                    .unwrap_or_else(|| self.make_word_range(position));
                return Some((symbol.qualified_name().to_string(), range));
            }
            // Target exists in index but couldn't be resolved (maybe unresolved import)
            // Return the target name as-is
            return Some((target_name.to_string(), self.make_word_range(position)));
        }

        // 2. Check if cursor is on a symbol definition
        for symbol in self.workspace.symbol_table().iter_symbols() {
            if symbol.source_file() == Some(&file_path_str) {
                if let Some(span) = symbol.span() {
                    if span.contains(pos) {
                        return Some((
                            symbol.qualified_name().to_string(),
                            span_to_lsp_range(&span),
                        ));
                    }
                }
            }
        }

        // Not on a reference or definition
        None
    }

    /// Create a default word range at the cursor position.
    /// Used when we know what symbol is referenced but don't have precise span info.
    fn make_word_range(&self, position: Position) -> Range {
        Range {
            start: position,
            end: Position {
                line: position.line,
                character: position.character + 1,
            },
        }
    }
}
