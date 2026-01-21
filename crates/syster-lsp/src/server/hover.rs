use super::helpers::{format_rich_hover, uri_to_path};
use super::LspServer;
use async_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Url};
use tracing::debug;

impl LspServer {
    /// Get hover information for a symbol at the given position
    ///
    /// Uses AST span tracking to find the exact element under the cursor,
    /// then provides rich information including relationships and documentation.
    pub fn get_hover(&self, uri: &Url, position: Position) -> Option<Hover> {
        debug!(
            "[HOVER] get_hover called for uri={}, position={}:{}",
            uri, position.line, position.character
        );

        let path = uri_to_path(uri)?;
        debug!("[HOVER] path={:?}", path);

        // Find symbol at position - returns qualified name string
        let result = self.find_symbol_at_position(&path, position);
        let (qualified_name, hover_range) = result?;

        // Look up the symbol using the resolver
        let resolver = self.resolver();
        let symbol = resolver.resolve(&qualified_name);
        debug!(
            "[HOVER] resolver.resolve({}) returned: {:?}",
            qualified_name,
            symbol.map(|s| s.qualified_name())
        );
        let symbol = symbol?;

        // Format rich hover content with relationships
        let content = format_rich_hover(symbol, self.workspace());

        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: Some(hover_range),
        })
    }
}
