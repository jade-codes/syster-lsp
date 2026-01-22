use crate::server::core::LspServer;
use async_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, InsertTextFormat, Position,
};

impl LspServer {
    /// Get completion items at a position
    ///
    /// Uses the new HIR-based IDE layer for completions.
    pub fn get_completions(
        &mut self,
        path: &std::path::Path,
        position: Position,
    ) -> CompletionResponse {
        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();
        
        // Get file ID for the new HIR layer
        let file_id = match analysis.get_file_id(&path_str) {
            Some(id) => id,
            None => return CompletionResponse::Array(Vec::new()),
        };

        // Determine trigger character from text
        let trigger = self.document_texts.get(path)
            .and_then(|text| {
                let lines: Vec<&str> = text.lines().collect();
                let line = lines.get(position.line as usize)?;
                let col = position.character as usize;
                if col > 0 {
                    line.chars().nth(col - 1)
                } else {
                    None
                }
            });

        // Use the Analysis completions method
        let ide_completions = analysis.completions(
            file_id,
            position.line,
            position.character,
            trigger,
        );

        // Convert to LSP CompletionItems
        let items: Vec<CompletionItem> = ide_completions
            .into_iter()
            .map(|item| {
                // Convert u32 kind to LSP CompletionItemKind
                let lsp_kind = match item.kind.to_lsp() {
                    9 => CompletionItemKind::MODULE,    // Package
                    7 => CompletionItemKind::CLASS,     // Definition
                    5 => CompletionItemKind::FIELD,     // Usage
                    14 => CompletionItemKind::KEYWORD,  // Keyword
                    15 => CompletionItemKind::SNIPPET,  // Snippet
                    _ => CompletionItemKind::TEXT,
                };
                let has_insert_text = item.insert_text.is_some();
                CompletionItem {
                    label: item.label.to_string(),
                    kind: Some(lsp_kind),
                    detail: item.detail.map(|s| s.to_string()),
                    documentation: item.documentation.map(|s| Documentation::String(s.to_string())),
                    insert_text: item.insert_text.map(|s| s.to_string()),
                    insert_text_format: if has_insert_text {
                        Some(InsertTextFormat::SNIPPET)
                    } else {
                        None
                    },
                    sort_text: Some(format!("{:03}_{}", item.sort_priority, item.label)),
                    ..Default::default()
                }
            })
            .collect();

        CompletionResponse::Array(items)
    }
}
