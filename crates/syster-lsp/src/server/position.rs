use super::LspServer;
use async_lsp::lsp_types::{Position, Range};
use std::path::Path;

impl LspServer {
    /// Find the symbol at the given position using semantic information.
    ///
    /// This uses the ReferenceIndex and SymbolTable populated during semantic analysis,
    /// avoiding fragile text extraction. The approach is:
    /// 1. Check if cursor is on a reference (usage site) → ReferenceIndex
    /// 2. Check if cursor is on a definition → SymbolTable
    pub fn find_symbol_at_position(
        &self,
        path: &Path,
        position: Position,
    ) -> Option<(String, Range)> {
        use super::helpers::span_to_lsp_range;

        let file_path_str = path.to_string_lossy().to_string();
        let pos = syster::core::Position::new(position.line as usize, position.character as usize);

        // 1. Check if cursor is on a reference (e.g., type annotation, specialization target)
        // Use get_full_reference_at_position to access scope_id for proper resolution
        if let Some((target_name, ref_info)) = self
            .workspace
            .reference_index()
            .get_full_reference_at_position(&file_path_str, pos)
        {
            // Resolve the target to get full symbol info
            let resolver = self.resolver();
            
            // Handle feature chains (e.g., lugNutCompositePort.lugNutPort1)
            // When chain_index > 0, we need to resolve via the chain context
            if let Some(ref chain_ctx) = ref_info.chain_context
                && chain_ctx.chain_index > 0
                && chain_ctx.chain_parts.len() > 1
            {
                // Resolve the first part of the chain, then resolve members
                let first_part = &chain_ctx.chain_parts[0];
                let scope_id = ref_info.scope_id.unwrap_or(0);
                
                if let Some(base_symbol) = resolver.resolve_in_scope(first_part, scope_id) {
                    // Walk through intermediate chain parts
                    let mut current_symbol = base_symbol;
                    for i in 1..chain_ctx.chain_index {
                        let part = &chain_ctx.chain_parts[i];
                        if let Some(next) = resolver.resolve_member(part, current_symbol, current_symbol.scope_id()) {
                            current_symbol = next;
                        } else {
                            break;
                        }
                    }
                    
                    // Resolve the final part (target_name) as a member
                    if let Some(symbol) = resolver.resolve_member(target_name, current_symbol, current_symbol.scope_id()) {
                        let range = symbol
                            .span()
                            .map(|s| span_to_lsp_range(&s))
                            .unwrap_or_else(|| self.make_word_range(position));
                        return Some((symbol.qualified_name().to_string(), range));
                    }
                }
            }
            
            // Standard resolution for non-chain or chain_index == 0
            let symbol = if let Some(scope_id) = ref_info.scope_id {
                resolver.resolve_in_scope(target_name, scope_id)
            } else {
                resolver.resolve(target_name)
            };
            
            if let Some(symbol) = symbol {
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
            if symbol.source_file() == Some(&file_path_str)
                && let Some(span) = symbol.span()
                && span.contains(pos)
            {
                return Some((
                    symbol.qualified_name().to_string(),
                    span_to_lsp_range(&span),
                ));
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
