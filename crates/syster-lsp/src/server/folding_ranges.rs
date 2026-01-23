//! Folding range support for the LSP server

use super::LspServer;
use async_lsp::lsp_types::{FoldingRange, FoldingRangeKind};
use std::path::Path;

impl LspServer {
    /// Get all foldable regions in a document using the new IDE layer
    pub fn get_folding_ranges(&mut self, file_path: &Path) -> Vec<FoldingRange> {
        let path_str = file_path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        let Some(file_id) = analysis.get_file_id(&path_str) else {
            return Vec::new();
        };

        // Use the Analysis folding_ranges method
        let ide_ranges = analysis.folding_ranges(file_id);

        // Convert to LSP FoldingRange
        let mut ranges: Vec<FoldingRange> = ide_ranges
            .into_iter()
            .map(|r| FoldingRange {
                start_line: r.start_line,
                start_character: Some(r.start_col),
                end_line: r.end_line,
                end_character: Some(r.end_col),
                kind: Some(if r.is_comment {
                    FoldingRangeKind::Comment
                } else {
                    FoldingRangeKind::Region
                }),
                collapsed_text: None,
            })
            .collect();

        ranges.sort_by_key(|r| r.start_line);
        ranges
    }
}
