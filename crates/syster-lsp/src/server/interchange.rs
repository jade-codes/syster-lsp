//! Interchange format support for LSP export/import commands.
//!
//! Provides custom LSP requests for exporting workspace models to XMI, KPAR, or JSON-LD
//! and importing models from these formats.

use super::LspServer;
use async_lsp::lsp_types::request::Request;
use serde::{Deserialize, Serialize};

// ============================================================================
// EXPORT REQUEST
// ============================================================================

/// Custom LSP request: syster/exportModel
///
/// Exports the current workspace to an interchange format.
pub enum ExportModelRequest {}

impl Request for ExportModelRequest {
    type Params = ExportModelParams;
    type Result = ExportModelResult;
    const METHOD: &'static str = "syster/exportModel";
}

/// Request parameters for syster/exportModel
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportModelParams {
    /// Output format: "xmi", "kpar", or "jsonld"
    pub format: String,

    /// Optional file URI to export (if None, exports entire workspace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Result of syster/exportModel
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportModelResult {
    /// Whether export succeeded
    pub success: bool,

    /// The exported data as base64-encoded bytes
    /// (base64 used because JSON can't represent raw bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    /// Suggested filename for saving
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Number of elements exported
    #[serde(default)]
    pub element_count: usize,

    /// Number of relationships exported
    #[serde(default)]
    pub relationship_count: usize,
}

// ============================================================================
// IMPORT REQUEST
// ============================================================================

/// Custom LSP request: syster/importModel
///
/// Imports a model from an interchange format and validates it.
pub enum ImportModelRequest {}

impl Request for ImportModelRequest {
    type Params = ImportModelParams;
    type Result = ImportModelResult;
    const METHOD: &'static str = "syster/importModel";
}

/// Request parameters for syster/importModel
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportModelParams {
    /// File URI to import
    pub uri: String,

    /// Optional format override (otherwise detected from extension)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Result of syster/importModel
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportModelResult {
    /// Whether import succeeded
    pub success: bool,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Number of elements imported
    #[serde(default)]
    pub element_count: usize,

    /// Number of relationships imported
    #[serde(default)]
    pub relationship_count: usize,

    /// Validation messages
    #[serde(default)]
    pub messages: Vec<String>,
}

// ============================================================================
// SERVER IMPLEMENTATION
// ============================================================================

#[cfg(feature = "interchange")]
impl LspServer {
    /// Export the workspace model to an interchange format.
    pub fn export_model(&mut self, params: &ExportModelParams) -> ExportModelResult {
        use base64::Engine;
        use syster::interchange::{JsonLd, Kpar, ModelFormat, Xmi, model_from_symbols};

        // Ensure workspace is loaded
        if let Err(e) = self.ensure_workspace_loaded() {
            return ExportModelResult {
                success: false,
                data: None,
                filename: None,
                error: Some(format!("Failed to load workspace: {}", e)),
                element_count: 0,
                relationship_count: 0,
            };
        }

        // Get all symbols from the analysis host
        let analysis = self.analysis_host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();

        // Convert to interchange model
        let model = model_from_symbols(&symbols);
        let element_count = model.elements.len();
        let relationship_count = model.relationship_count();

        // Serialize to requested format
        let (bytes_result, extension) = match params.format.to_lowercase().as_str() {
            "xmi" => (Xmi.write(&model), "xmi"),
            "kpar" => (Kpar.write(&model), "kpar"),
            "jsonld" | "json-ld" => (JsonLd.write(&model), "jsonld"),
            _ => {
                return ExportModelResult {
                    success: false,
                    data: None,
                    filename: None,
                    error: Some(format!(
                        "Unsupported format: {}. Use xmi, kpar, or jsonld.",
                        params.format
                    )),
                    element_count,
                    relationship_count,
                };
            }
        };

        match bytes_result {
            Ok(bytes) => {
                // Encode as base64 for JSON transport
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                ExportModelResult {
                    success: true,
                    data: Some(data),
                    filename: Some(format!("model.{}", extension)),
                    error: None,
                    element_count,
                    relationship_count,
                }
            }
            Err(e) => ExportModelResult {
                success: false,
                data: None,
                filename: None,
                error: Some(format!("Export failed: {}", e)),
                element_count,
                relationship_count,
            },
        }
    }

    /// Import and validate a model from an interchange format.
    pub fn import_model(&mut self, params: &ImportModelParams) -> ImportModelResult {
        use syster::interchange::{JsonLd, Kpar, ModelFormat, Xmi, detect_format};

        // Parse the URI to get the file path
        let path = match async_lsp::lsp_types::Url::parse(&params.uri)
            .ok()
            .and_then(|url| url.to_file_path().ok())
        {
            Some(p) => p,
            None => {
                return ImportModelResult {
                    success: false,
                    error: Some(format!("Invalid URI: {}", params.uri)),
                    element_count: 0,
                    relationship_count: 0,
                    messages: Vec::new(),
                };
            }
        };

        // Read the file
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                return ImportModelResult {
                    success: false,
                    error: Some(format!("Failed to read file: {}", e)),
                    element_count: 0,
                    relationship_count: 0,
                    messages: Vec::new(),
                };
            }
        };

        // Determine format
        let format_str = params
            .format
            .as_deref()
            .unwrap_or_else(|| path.extension().and_then(|e| e.to_str()).unwrap_or("xmi"));

        // Parse the model
        let model = match format_str.to_lowercase().as_str() {
            "xmi" => Xmi.read(&bytes),
            "kpar" => Kpar.read(&bytes),
            "jsonld" | "json-ld" | "json" => JsonLd.read(&bytes),
            _ => {
                if let Some(format_impl) = detect_format(&path) {
                    format_impl.read(&bytes)
                } else {
                    return ImportModelResult {
                        success: false,
                        error: Some(format!("Unknown format: {}", format_str)),
                        element_count: 0,
                        relationship_count: 0,
                        messages: Vec::new(),
                    };
                }
            }
        };

        match model {
            Ok(model) => {
                let mut messages = Vec::new();

                // Validate relationships reference existing elements
                for rel in model.iter_relationship_elements() {
                    if let Some(rd) = &rel.relationship {
                        if let Some(src) = rd.source.first() {
                            if model.elements.get(src).is_none() {
                                messages.push(format!(
                                    "Warning: Relationship source '{}' not found",
                                    src
                                ));
                            }
                        }
                        if let Some(tgt) = rd.target.first() {
                            if model.elements.get(tgt).is_none() {
                                messages.push(format!(
                                    "Warning: Relationship target '{}' not found",
                                    tgt
                                ));
                            }
                        }
                    }
                }

                let element_count = model.elements.len();
                let relationship_count = model.relationship_count();

                // Inject the model into the analysis host so it becomes
                // available for IDE features (go-to-definition, etc.)
                let virtual_path = format!(
                    "imported://{}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("model.sysml")
                );
                let parse_errors = self.analysis_host.add_model(&model, &virtual_path);
                for err in &parse_errors {
                    messages.push(format!("Parse warning: {}", err));
                }

                ImportModelResult {
                    success: true,
                    error: None,
                    element_count,
                    relationship_count,
                    messages,
                }
            }
            Err(e) => ImportModelResult {
                success: false,
                error: Some(format!("Parse failed: {}", e)),
                element_count: 0,
                relationship_count: 0,
                messages: Vec::new(),
            },
        }
    }
}
