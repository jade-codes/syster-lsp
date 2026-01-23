use async_lsp::lsp_types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use syster::core::ParseError;
use syster::core::constants::{
    COMPLETION_TRIGGERS, LSP_SERVER_NAME, LSP_SERVER_VERSION, OPT_STDLIB_ENABLED, OPT_STDLIB_PATH,
};
use syster::ide::AnalysisHost;
use syster::project::{StdLibLoader, WorkspaceLoader};
use tokio_util::sync::CancellationToken;

/// LspServer manages the workspace state for the LSP server
pub struct LspServer {
    /// Unified analysis host - holds workspace, symbol index, and file maps
    pub(super) analysis_host: AnalysisHost,
    /// Track parse errors for each file (keyed by file path)
    pub(super) parse_errors: HashMap<PathBuf, Vec<ParseError>>,
    /// Track document text for hover and other features (keyed by file path)
    pub(super) document_texts: HashMap<PathBuf, String>,
    /// Stdlib loader for lazy loading
    pub(super) stdlib_loader: StdLibLoader,
    /// Whether stdlib loading is enabled
    stdlib_enabled: bool,
    /// Cancellation tokens per document - cancelled when document changes
    document_cancel_tokens: HashMap<PathBuf, CancellationToken>,
    /// Whether workspace has been fully initialized
    workspace_initialized: bool,
    /// Workspace folders to scan for SysML/KerML files
    workspace_folders: Vec<PathBuf>,
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    /// Returns the server capabilities for LSP initialization
    pub fn server_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                        include_text: Some(false),
                    })),
                },
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            document_formatting_provider: Some(OneOf::Left(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(
                    COMPLETION_TRIGGERS.iter().map(|s| s.to_string()).collect(),
                ),
                ..Default::default()
            }),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(false),
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    legend: Self::semantic_tokens_legend(),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    range: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
            ),
            document_link_provider: Some(DocumentLinkOptions {
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: None,
                file_operations: None,
            }),
            ..Default::default()
        }
    }

    /// Returns the InitializeResult for the LSP handshake
    pub fn initialize_result() -> InitializeResult {
        InitializeResult {
            capabilities: Self::server_capabilities(),
            server_info: Some(ServerInfo {
                name: LSP_SERVER_NAME.to_string(),
                version: Some(LSP_SERVER_VERSION.to_string()),
            }),
        }
    }

    /// Parse initialization options from the client
    pub fn parse_init_options(options: Option<serde_json::Value>) -> (bool, Option<PathBuf>) {
        if let Some(serde_json::Value::Object(opts)) = options {
            let enabled = opts
                .get(OPT_STDLIB_ENABLED)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let path = opts
                .get(OPT_STDLIB_PATH)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(PathBuf::from);
            (enabled, path)
        } else {
            (true, None)
        }
    }

    pub fn new() -> Self {
        Self::with_config(true, None)
    }

    /// Create a new LspServer with custom configuration
    pub fn with_config(stdlib_enabled: bool, custom_stdlib_path: Option<PathBuf>) -> Self {
        // Use custom path or let StdLibLoader discover it automatically
        let stdlib_loader = match custom_stdlib_path {
            Some(path) => StdLibLoader::with_path(path),
            None => StdLibLoader::new(),
        };

        Self {
            analysis_host: AnalysisHost::new(),
            parse_errors: HashMap::new(),
            document_texts: HashMap::new(),
            stdlib_loader,
            stdlib_enabled,
            document_cancel_tokens: HashMap::new(),
            workspace_initialized: false,
            workspace_folders: Vec::new(),
        }
    }

    /// Set the workspace folders to scan for SysML/KerML files
    pub fn set_workspace_folders(&mut self, folders: Vec<PathBuf>) {
        self.workspace_folders = folders;
    }

    /// Ensure workspace is fully initialized (stdlib loaded, symbols populated, texts synced).
    /// Only runs once on first call, subsequent calls are no-ops.
    ///
    /// Parse errors in individual files are logged but don't block workspace loading.
    /// Valid files are still loaded and functional even when some files have parse errors.
    pub fn ensure_workspace_loaded(&mut self) -> Result<(), String> {
        if self.workspace_initialized {
            return Ok(());
        }

        // Load stdlib if enabled
        if self.stdlib_enabled {
            self.stdlib_loader
                .ensure_loaded_into_host(&mut self.analysis_host)?;
        }

        // Load all SysML/KerML files from workspace folders
        // Parse errors are collected but don't block loading of valid files
        let loader = WorkspaceLoader::new();
        for folder in self.workspace_folders.clone() {
            if let Err(err) = loader.load_directory_into_host(&folder, &mut self.analysis_host) {
                // Log parse errors but continue - valid files are already loaded
                tracing::warn!(
                    folder = %folder.display().to_string(),
                    "Some files failed to parse: {err}"
                );
            }
        }

        // Sync document texts for hover/features on loaded files
        self.sync_document_texts_from_files();

        // Mark dirty so index is rebuilt on next analysis() call
        self.analysis_host.mark_dirty();

        self.workspace_initialized = true;
        Ok(())
    }

    /// Cancel any in-flight operations for a document and return a new token.
    /// Call this at the start of didChange to cancel previous operations.
    pub fn cancel_document_operations(&mut self, path: &PathBuf) -> CancellationToken {
        // Cancel the old token if it exists
        if let Some(old_token) = self.document_cancel_tokens.get(path) {
            old_token.cancel();
        }
        // Create a new token for this document
        let new_token = CancellationToken::new();
        self.document_cancel_tokens
            .insert(path.clone(), new_token.clone());
        new_token
    }

    /// Get the current cancellation token for a document (for request handlers)
    pub fn get_document_cancel_token(&self, path: &PathBuf) -> Option<CancellationToken> {
        self.document_cancel_tokens.get(path).cloned()
    }

    /// Sync document_texts with all files currently loaded
    /// This ensures hover and other features work on all files without disk reads
    fn sync_document_texts_from_files(&mut self) {
        for path in self.analysis_host.files().keys() {
            // Only load if not already tracked (avoid overwriting editor versions)
            if !self.document_texts.contains_key(path) {
                if let Ok(text) = std::fs::read_to_string(path) {
                    self.document_texts.insert(path.clone(), text);
                }
            }
        }
    }

    // ==================== Accessors ====================

    /// Get the number of files loaded
    #[allow(dead_code)]
    pub fn file_count(&self) -> usize {
        self.analysis_host.file_count()
    }

    /// Get mutable access to document_texts
    #[allow(dead_code)]
    pub fn document_texts_mut(&mut self) -> &mut HashMap<PathBuf, String> {
        &mut self.document_texts
    }
}
