//! Type information request handler for LSP.
//!
//! Provides detailed information about type references at a cursor position.
//! This is a custom LSP request that exposes the syster-base type_info feature.

use super::LspServer;
use super::helpers::uri_to_path;
use async_lsp::lsp_types::request::Request;
use async_lsp::lsp_types::{Position, Url};
use serde::{Deserialize, Serialize};

/// Custom LSP request: syster/typeInfo
///
/// Returns type information when the cursor is on a type reference.
pub enum TypeInfoRequest {}

impl Request for TypeInfoRequest {
    type Params = TypeInfoParams;
    type Result = Option<TypeInfoResult>;
    const METHOD: &'static str = "syster/typeInfo";
}

/// Request parameters for syster/typeInfo
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeInfoParams {
    /// URI of the document
    pub uri: String,
    /// Cursor position
    pub position: Position,
}

/// Result of the syster/typeInfo request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeInfoResult {
    /// The type name as written in source (may be simple or qualified)
    pub target_name: String,

    /// The fully resolved qualified name (if resolved)
    pub resolved_name: Option<String>,

    /// The kind of the resolved target (e.g., "part def", "item def")
    pub target_kind: Option<String>,

    /// Documentation of the resolved target
    pub target_doc: Option<String>,

    /// The containing symbol's qualified name
    pub container: Option<String>,

    /// The kind of type reference (typed_by, specializes, subsets, etc.)
    pub ref_kind: String,

    /// Start line of the type reference (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_col: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_col: u32,
}

impl LspServer {
    /// Get type information at a position.
    ///
    /// Returns info if the cursor is on a type annotation (`:`, `:>`, `::>`, etc.).
    pub fn get_type_info(&mut self, uri: &Url, position: Position) -> Option<TypeInfoResult> {
        let path = uri_to_path(uri)?;
        let path_str = path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        // Get file ID
        let file_id = analysis.get_file_id(&path_str)?;

        // Use the type_info_at function from syster-base
        let info = analysis.type_info_at(file_id, position.line, position.character)?;

        Some(TypeInfoResult {
            target_name: info.target_name.to_string(),
            resolved_name: info
                .resolved_symbol
                .as_ref()
                .map(|s| s.qualified_name.to_string()),
            target_kind: info
                .resolved_symbol
                .as_ref()
                .map(|s| s.kind.display().to_string()),
            target_doc: info
                .resolved_symbol
                .as_ref()
                .and_then(|s| s.doc.as_ref().map(|d| d.to_string())),
            container: info.container.map(|c| c.to_string()),
            ref_kind: info.type_ref.kind.display().to_string(),
            start_line: info.type_ref.start_line,
            start_col: info.type_ref.start_col,
            end_line: info.type_ref.end_line,
            end_col: info.type_ref.end_col,
        })
    }
}
