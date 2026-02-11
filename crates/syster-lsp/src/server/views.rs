//! SysML v2 Views request handler for LSP.
//!
//! Provides access to user-defined SysML v2 Views that can filter and expose
//! specific elements in the workspace according to stakeholder concerns.

use super::LspServer;
use async_lsp::lsp_types::request::Request;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Custom LSP request: syster/getSysMLViews
///
/// Discovers all ViewDefinitions in the workspace and optionally applies them
/// to return the filtered list of visible symbols.
pub enum GetSysMLViewsRequest {}

impl Request for GetSysMLViewsRequest {
    type Params = GetSysMLViewsParams;
    type Result = GetSysMLViewsResult;
    const METHOD: &'static str = "syster/getSysMLViews";
}

/// Request parameters for syster/getSysMLViews
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSysMLViewsParams {
    /// Optional: qualified name of a specific view to apply
    /// If None, returns all discovered views without applying them
    pub view_name: Option<String>,

    /// Optional: URI of the file being viewed
    /// Used to auto-expose file contents when view has no explicit expose
    pub uri: Option<String>,
}

/// Result of the syster/getSysMLViews request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSysMLViewsResult {
    /// All discovered view definitions in the workspace
    pub views: Vec<ViewInfo>,

    /// If a specific view was requested, the filtered symbols
    pub visible_symbols: Option<Vec<String>>,
}

/// Information about a discovered view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewInfo {
    /// Qualified name of the view (e.g., "Views::VehicleStructureView")
    pub qualified_name: String,

    /// Simple name (e.g., "VehicleStructureView")
    pub name: String,

    /// Expose relationships in this view
    pub expose: Vec<ExposeInfo>,

    /// Filter conditions in this view
    pub filters: Vec<FilterInfo>,

    /// Documentation for this view
    pub documentation: Option<String>,

    /// File where this view is defined
    pub file_path: Option<String>,
}

/// Information about an expose relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposeInfo {
    /// Target qualified name
    pub target: String,

    /// Wildcard kind: "none", "direct", or "recursive"
    pub wildcard: String,
}

/// Information about a filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterInfo {
    /// Filter type: "metadata" or "expression"
    pub filter_type: String,

    /// The filter value (metadata annotation or expression)
    pub value: String,
}

impl LspServer {
    /// Get all SysML Views in the workspace.
    ///
    /// If a specific view name is provided, applies that view and returns
    /// the filtered list of visible symbols.
    pub fn get_sysml_views(&mut self, params: &GetSysMLViewsParams) -> GetSysMLViewsResult {
        // Ensure the workspace (including standard library) is loaded
        if let Err(e) = self.ensure_workspace_loaded() {
            info!("[Views] Failed to load workspace: {}", e);
            return GetSysMLViewsResult {
                views: Vec::new(),
                visible_symbols: None,
            };
        }

        // Get the analysis and symbol index
        let _analysis = self.analysis_host.analysis();
        let symbol_index = self.analysis_host.symbol_index();

        // Debug: Count symbols and find views
        let total_symbols = symbol_index.all_symbols().count();
        info!("[Views] Total symbols in index: {}", total_symbols);

        // Debug: Find all symbols with view_data
        let symbols_with_view_data: Vec<_> = symbol_index
            .all_symbols()
            .filter(|s| s.view_data.is_some())
            .collect();
        info!(
            "[Views] Symbols with view_data: {}",
            symbols_with_view_data.len()
        );

        for s in &symbols_with_view_data {
            info!(
                "[Views]   - {} (kind={:?}, name={})",
                s.qualified_name, s.kind, s.name
            );
        }

        // Debug: Find ViewDefinition symbols specifically
        let view_def_symbols: Vec<_> = symbol_index
            .all_symbols()
            .filter(|s| matches!(s.kind, syster::hir::SymbolKind::ViewDefinition))
            .collect();
        info!(
            "[Views] ViewDefinition kind symbols: {}",
            view_def_symbols.len()
        );

        for s in &view_def_symbols {
            info!(
                "[Views]   ViewDef: qn={}, name={}, has_view_data={}",
                s.qualified_name,
                s.name,
                s.view_data.is_some()
            );
        }

        // Standard view types from StandardViewDefinitions.sysml
        // These are the ones useful for diagram visualization
        // Format: (short_name, display_name) - short_name is in the symbol, display_name is what users see
        // The SysML v2 syntax `view def <gv> GeneralView` stores the short name "gv" as the symbol name
        const STANDARD_VIEWS: &[(&str, &str)] = &[
            ("gv", "General View"),
            ("iv", "Interconnection View (IBD)"),
            ("afv", "Action Flow View"),
            ("stv", "State Transition View"),
            ("sv", "Sequence View"),
            ("gev", "Geometry View"),
            ("grv", "Grid View"),
            ("bv", "Browser View"),
        ];

        // Collect ViewDefinition symbols, only standard views
        let mut views = Vec::new();

        for symbol in symbol_index.all_symbols() {
            // Check if this is a ViewDefinition - use SymbolKind instead of view_data
            if matches!(symbol.kind, syster::hir::SymbolKind::ViewDefinition) {
                // Get the short name (e.g., "gv" from `view def <gv> GeneralView`)
                let short_name = symbol.short_name.as_deref().unwrap_or("");

                info!(
                    "[Views] Checking ViewDef: qn={}, name={}, short_name={}",
                    symbol.qualified_name, symbol.name, short_name
                );

                // Only include standard views - match by short name
                if let Some((_, display_name)) =
                    STANDARD_VIEWS.iter().find(|(sn, _)| *sn == short_name)
                {
                    info!("[Views] MATCHED standard view: {}", display_name);

                    // Get view_data if available for expose/filter info
                    let (expose, filters) =
                        if let Some(syster::hir::ViewData::ViewDefinition(view_def)) =
                            &symbol.view_data
                        {
                            let expose = view_def
                                .expose
                                .iter()
                                .map(|exp| ExposeInfo {
                                    target: exp.target().to_string(),
                                    wildcard: match exp.import_path.wildcard {
                                        syster::hir::WildcardKind::None => "none".to_string(),
                                        syster::hir::WildcardKind::Direct => "direct".to_string(),
                                        syster::hir::WildcardKind::Recursive => {
                                            "recursive".to_string()
                                        }
                                    },
                                })
                                .collect();

                            let filters = view_def
                                .filter
                                .iter()
                                .map(|filter| match filter {
                                    syster::hir::FilterCondition::Metadata(meta) => FilterInfo {
                                        filter_type: "metadata".to_string(),
                                        value: meta.annotation.to_string(),
                                    },
                                    syster::hir::FilterCondition::Expression(expr) => FilterInfo {
                                        filter_type: "expression".to_string(),
                                        value: expr.clone(),
                                    },
                                })
                                .collect();
                            (expose, filters)
                        } else {
                            (Vec::new(), Vec::new())
                        };

                    // Get file path from FileId
                    let file_path = self
                        .analysis_host
                        .get_file_path(symbol.file)
                        .map(|p| p.to_string());

                    views.push(ViewInfo {
                        qualified_name: symbol.qualified_name.to_string(),
                        name: display_name.to_string(), // Use friendly display name
                        expose,
                        filters,
                        documentation: symbol.doc.as_ref().map(|d: &Arc<str>| d.to_string()),
                        file_path,
                    });
                }
            }
        }

        info!("[Views] Final views count: {}", views.len());

        // Sort views in the defined order
        views.sort_by(|a, b| {
            let a_idx = STANDARD_VIEWS
                .iter()
                .position(|(_, display)| a.name == *display);
            let b_idx = STANDARD_VIEWS
                .iter()
                .position(|(_, display)| b.name == *display);
            match (a_idx, b_idx) {
                (Some(ai), Some(bi)) => ai.cmp(&bi),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.cmp(&b.name),
            }
        });

        // If a specific view was requested, apply it
        let visible_symbols = if let Some(view_name) = &params.view_name {
            self.apply_view(view_name, params.uri.as_deref())
        } else {
            None
        };

        GetSysMLViewsResult {
            views,
            visible_symbols,
        }
    }

    /// Apply a specific view to the workspace symbols.
    ///
    /// Standard view types (from SysML v2 spec StandardViewDefinitions.sysml):
    /// - GeneralView: Shows any/all model elements
    /// - InterconnectionView: Features as nodes, connections as edges
    /// - ActionFlowView: Actions, parameters, flow connections, control nodes
    /// - StateTransitionView: States, transitions, entry/do/exit actions
    /// - SequenceView: Lifelines, event occurrences, messages
    /// - BrowserView: Hierarchical membership structure
    fn apply_view(&mut self, view_name: &str, uri: Option<&str>) -> Option<Vec<String>> {
        info!(
            "[Views] apply_view called: view_name={}, uri={:?}",
            view_name, uri
        );

        let analysis = self.analysis_host.analysis();
        let symbol_index = analysis.symbol_index();

        // Collect symbols from the file (or all if no URI)
        let file_symbols: Vec<_> = if let Some(uri_str) = uri {
            // Convert URI to file path (get_file_id expects path, not URI)
            let path = if uri_str.starts_with("file://") {
                async_lsp::lsp_types::Url::parse(uri_str)
                    .ok()
                    .and_then(|u| u.to_file_path().ok())
                    .map(|p| p.to_string_lossy().to_string())
            } else {
                Some(uri_str.to_string())
            };

            info!("[Views] Looking for file_id for path: {:?}", path);

            if let Some(path_str) = path {
                if let Some(file_id) = analysis.get_file_id(&path_str) {
                    info!("[Views] Found file_id: {:?}", file_id);
                    let syms: Vec<_> = symbol_index.symbols_in_file(file_id);
                    info!("[Views] symbols_in_file returned {} symbols", syms.len());
                    syms
                } else {
                    info!("[Views] No file_id found for path, returning empty");
                    return Some(Vec::new());
                }
            } else {
                info!("[Views] Could not parse URI to path, returning empty");
                return Some(Vec::new());
            }
        } else {
            info!("[Views] No URI provided, using all symbols");
            symbol_index.all_symbols().collect()
        };

        info!("[Views] file_symbols count: {}", file_symbols.len());

        // Apply standard view filtering based on view type name
        let filtered = filter_symbols_for_view(&file_symbols, view_name);
        info!("[Views] filtered count: {}", filtered.len());
        Some(filtered)
    }
}

/// Filter symbols based on the standard view type.
///
/// From SysML v2 StandardViewDefinitions.sysml:
/// - GeneralView: Any members of exposed model elements
/// - InterconnectionView: Features as nodes, nested features as nested nodes, connections as edges
/// - ActionFlowView: Actions, parameters, flow connections, control nodes
/// - StateTransitionView: States, transitions, entry/do/exit actions  
/// - SequenceView: Lifelines, event occurrences, messages
/// - BrowserView: Hierarchical membership structure
fn filter_symbols_for_view(symbols: &[&syster::hir::HirSymbol], view_name: &str) -> Vec<String> {
    use syster::hir::SymbolKind;

    // Extract the simple view type name from qualified name
    // e.g., "StandardViewDefinitions::iv" -> "iv"
    let short_name = view_name.rsplit("::").next().unwrap_or(view_name);

    // Map short names to view types
    // The SysML v2 stdlib uses short names like <gv>, <iv>, etc.
    let view_type = match short_name {
        "gv" => "GeneralView",
        "iv" => "InterconnectionView",
        "afv" => "ActionFlowView",
        "stv" => "StateTransitionView",
        "sv" => "SequenceView",
        "gev" => "GeometryView",
        "grv" => "GridView",
        "bv" => "BrowserView",
        // Fallback: use the name as-is (in case full names are used)
        other => other,
    };

    info!(
        "[Views] filter_symbols_for_view: view_name={}, short_name={}, view_type={}",
        view_name, short_name, view_type
    );
    info!(
        "[Views] filter_symbols_for_view: view_name={}, short_name={}, view_type={}",
        view_name, short_name, view_type
    );

    let filtered: Vec<String> = symbols
        .iter()
        .filter(|symbol| {
            match view_type {
                // GeneralView: show everything (any model element)
                "GeneralView" => true,

                // InterconnectionView: features as nodes, connections as edges
                // "present exposed features as nodes, nested features as nested nodes,
                //  and connections between features as edges"
                // NOTE: Definitions are TYPE definitions, not instances
                // An IBD shows the internal structure of a specific context (usages only)
                "InterconnectionView" => matches!(
                    symbol.kind,
                    // Structural usages that become nodes (the "parts" in the diagram)
                    SymbolKind::PartUsage
                        | SymbolKind::PortUsage
                        | SymbolKind::InterfaceUsage
                        // Connections/flows that become edges (filtered out as nodes in webview)
                        | SymbolKind::ConnectionUsage
                        | SymbolKind::FlowConnectionUsage
                        | SymbolKind::AllocationUsage
                ),

                // ActionFlowView: actions, parameters, flow connections, control nodes
                // "present connections between actions"
                "ActionFlowView" => matches!(
                    symbol.kind,
                    // Actions
                    SymbolKind::ActionUsage
                        | SymbolKind::ActionDefinition
                        // Parameters
                        | SymbolKind::AttributeUsage
                        | SymbolKind::ReferenceUsage
                        // Flow connections
                        | SymbolKind::FlowConnectionUsage
                        | SymbolKind::ConnectionUsage
                        // Items being transferred
                        | SymbolKind::ItemUsage
                ),

                // StateTransitionView: states and transitions
                // "present states and their transitions"
                "StateTransitionView" => matches!(
                    symbol.kind,
                    // States
                    SymbolKind::StateUsage
                        | SymbolKind::StateDefinition
                        // Entry/do/exit are actions
                        | SymbolKind::ActionUsage
                        // Transitions are succession usages
                        | SymbolKind::ConnectionUsage
                ),

                // SequenceView: lifelines, event occurrences, messages
                // "present time ordering of event occurrences on lifelines"
                "SequenceView" => matches!(
                    symbol.kind,
                    // Features with lifelines
                    SymbolKind::PartUsage
                        | SymbolKind::ItemUsage
                        // Event occurrences (occurrence usages)
                        | SymbolKind::OccurrenceUsage
                        // Messages
                        | SymbolKind::FlowConnectionUsage
                        | SymbolKind::ActionUsage
                ),

                // BrowserView: hierarchical membership structure
                // "present the hierarchical membership structure"
                "BrowserView" => matches!(
                    symbol.kind,
                    // All structural elements
                    SymbolKind::Package
                        | SymbolKind::PartDefinition
                        | SymbolKind::PartUsage
                        | SymbolKind::ItemDefinition
                        | SymbolKind::ItemUsage
                        | SymbolKind::ActionDefinition
                        | SymbolKind::ActionUsage
                        | SymbolKind::StateDefinition
                        | SymbolKind::StateUsage
                        | SymbolKind::RequirementDefinition
                        | SymbolKind::RequirementUsage
                        | SymbolKind::ConstraintDefinition
                        | SymbolKind::ConstraintUsage
                        | SymbolKind::ViewDefinition
                        | SymbolKind::ViewpointDefinition
                ),

                // GridView: tabular/matrix view - show everything for now
                "GridView" => true,

                // GeometryView: spatial items - not supported yet, show everything
                "GeometryView" => true,

                // Unknown view type: show everything
                _ => true,
            }
        })
        .map(|s| s.qualified_name.to_string())
        .collect();

    filtered
}
