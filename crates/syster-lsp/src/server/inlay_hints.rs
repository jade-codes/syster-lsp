//! Inlay hint support for the LSP server

use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, Position as LspPosition,
};
use syster::ide;

impl LspServer {
    /// Get inlay hints for a document
    pub fn get_inlay_hints(&mut self, params: &InlayHintParams) -> Vec<InlayHint> {
        let uri = &params.text_document.uri;

        let Some(path) = uri_to_path(uri) else {
            return vec![];
        };

        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        let Some(file_id) = analysis.get_file_id(&path_str) else {
            return vec![];
        };

        // Convert LSP range to tuple of (start_line, start_col, end_line, end_col)
        let range = Some((
            params.range.start.line,
            params.range.start.character,
            params.range.end.line,
            params.range.end.character,
        ));

        // Extract hints using the Analysis inlay_hints method
        let hints = analysis.inlay_hints(file_id, range);

        // Convert IDE hints to LSP hints
        hints
            .into_iter()
            .map(|hint| InlayHint {
                position: LspPosition {
                    line: hint.line,
                    character: hint.col,
                },
                label: InlayHintLabel::String(hint.label),
                kind: Some(match hint.kind {
                    ide::InlayHintKind::Type => InlayHintKind::TYPE,
                    ide::InlayHintKind::Parameter => InlayHintKind::PARAMETER,
                }),
                text_edits: None,
                tooltip: None,
                padding_left: Some(hint.padding_left),
                padding_right: Some(hint.padding_right),
                data: None,
            })
            .collect()
    }
}
