use super::LspServer;
use super::helpers::{format_rich_hover, uri_to_path};
use async_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Url};

impl LspServer {
    /// Get hover information for a symbol at the given position
    ///
    /// Uses AST span tracking to find the exact element under the cursor,
    /// then provides rich information including relationships and documentation.
    pub fn get_hover(&self, uri: &Url, position: Position) -> Option<Hover> {
        let path = uri_to_path(uri)?;

        // Find symbol at position - returns qualified name string
        let result = self.find_symbol_at_position(&path, position);
        if position.line == 1472 { // Line 1473 is 0-indexed as 1472

        }
        let (qualified_name, hover_range) = result?;

        // Look up the symbol using the resolver
        let resolver = self.resolver();
        let symbol = resolver.resolve(&qualified_name)?;

        // Format rich hover content with relationships
        let content = format_rich_hover(symbol, self.workspace());

        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: Some(hover_range),
        })
    }
}
