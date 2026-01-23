use super::LspServer;
use super::helpers::{position_to_lsp_position, uri_to_path};
use async_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use syster::hir::{check_file, Severity as HirSeverity};

impl LspServer {
    /// Get LSP diagnostics for a given file (parse errors + semantic errors)
    pub fn get_diagnostics(&mut self, uri: &Url) -> Vec<Diagnostic> {
        let Some(path) = uri_to_path(uri) else {
            return vec![];
        };

        let mut diagnostics = Vec::new();

        // 1. Convert parse errors to LSP diagnostics
        if let Some(errors) = self.parse_errors.get(&path) {
            for e in errors.iter() {
                let pos = position_to_lsp_position(&e.position);
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: pos,
                        end: Position {
                            line: pos.line,
                            character: pos.character + 1,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e.message.clone(),
                    source: Some("syster-parse".to_string()),
                    ..Default::default()
                });
            }
        }

        // 2. Add semantic diagnostics (only if no parse errors - semantic checks need valid AST)
        if diagnostics.is_empty() {
            let analysis = self.analysis_host.analysis();
            let path_str = path.to_string_lossy();
            if let Some(file_id) = analysis.get_file_id(&path_str) {
                let semantic_diags = check_file(analysis.symbol_index(), file_id);
                for diag in semantic_diags {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: diag.start_line,
                                character: diag.start_col,
                            },
                            end: Position {
                                line: diag.end_line,
                                character: diag.end_col,
                            },
                        },
                        severity: Some(hir_severity_to_lsp(diag.severity)),
                        code: diag.code.map(|c| async_lsp::lsp_types::NumberOrString::String(c.to_string())),
                        message: diag.message.to_string(),
                        source: Some("syster-semantic".to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }
}

/// Convert HIR severity to LSP severity
fn hir_severity_to_lsp(severity: HirSeverity) -> DiagnosticSeverity {
    match severity {
        HirSeverity::Error => DiagnosticSeverity::ERROR,
        HirSeverity::Warning => DiagnosticSeverity::WARNING,
        HirSeverity::Info => DiagnosticSeverity::INFORMATION,
        HirSeverity::Hint => DiagnosticSeverity::HINT,
    }
}
