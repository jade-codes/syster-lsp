//! Diagram data provider for VS Code webview integration.
//!
//! Provides diagram data (symbols + relationships) in a format consumable
//! by the diagram-core TypeScript package.
//!
//! The viewer expects symbols with `kind` ("Definition", "Usage", "Package"),
//! `definitionKind` ("Part", "Port", etc.), and `usageKind` fields.
//! These are combined by the viewer to form node types like "PartDef", "PartUsage".

use super::LspServer;
use async_lsp::lsp_types::request::Request;
use serde::{Deserialize, Serialize};
use std::path::Path;
use syster::hir::{HirSymbol, SymbolKind};

/// Custom LSP request: syster/getDiagram
pub enum GetDiagramRequest {}

impl Request for GetDiagramRequest {
    type Params = GetDiagramParams;
    type Result = DiagramData;
    const METHOD: &'static str = "syster/getDiagram";
}

/// Request parameters for syster/getDiagram
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiagramParams {
    /// URI of the file to get diagram for (optional - if None, returns whole workspace)
    pub uri: Option<String>,

    /// View type to use for rendering (from StandardViewDefinitions)
    /// Defaults to "GeneralView" if not specified
    #[serde(default = "default_view_type")]
    pub view_type: String,
}

fn default_view_type() -> String {
    "GeneralView".to_string()
}

/// Symbol data for diagram visualization.
///
/// The viewer expects a specific schema with `kind`, `definitionKind`, and `usageKind`
/// fields to properly determine node types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramSymbol {
    /// Simple name of the element
    pub name: String,

    /// Fully qualified name (e.g., "Package::Element")
    pub qualified_name: String,

    /// High-level kind: "Definition", "Usage", "Package", "Feature", "Classifier"
    pub kind: String,

    /// For definitions: "Part", "Port", "Action", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_kind: Option<String>,

    /// For usages: "Part", "Port", "Action", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_kind: Option<String>,

    /// Parent's qualified name for containment hierarchy.
    /// Used by React Flow to create nested/grouped nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    /// Optional features to display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,

    /// Type that this usage is typed by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed_by: Option<String>,

    /// Direction for ports/features: "in", "out", "inout"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

/// Relationship data for diagram edges
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramRelationship {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub source: String,
    pub target: String,
}

/// Complete diagram data response.
///
/// When `error` is `Some`, the view could not be rendered and `symbols`/
/// `relationships` are empty. The frontend must surface the error rather than
/// render an empty or fallback diagram — a view that cannot be applied is an
/// error the user needs to see, not something to paper over.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramData {
    pub symbols: Vec<DiagramSymbol>,
    pub relationships: Vec<DiagramRelationship>,
    pub view_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DiagramError>,
}

/// A structured, surfaceable error explaining why a view could not be rendered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramError {
    /// Machine-readable kind: "UnsupportedView" | "UnknownView" | "NoFile".
    pub kind: String,
    /// Human-readable message for display in the webview.
    pub message: String,
}

/// The view kinds this build can actually render. Anything else is reported as
/// an error instead of being silently rendered as a generic graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewKind {
    General,
    Interconnection,
    Browser,
}

/// Resolve a requested view type (accepting either a display/full name like
/// "GeneralView"/"InterconnectionView"/"BrowserView" or a stdlib short name
/// like "gv"/"iv"/"bv") to a renderable [`ViewKind`], or a [`DiagramError`]
/// explaining why it cannot be rendered.
///
/// Known-but-unimplemented standard views (Action Flow, State Transition,
/// Sequence, Geometry, Grid) return `UnsupportedView`; anything unrecognized
/// returns `UnknownView`. There is intentionally **no** silent fallback.
fn resolve_view_kind(view_type: &str) -> Result<ViewKind, DiagramError> {
    let short = view_type.rsplit("::").next().unwrap_or(view_type);
    match short {
        "GeneralView" | "gv" => Ok(ViewKind::General),
        "InterconnectionView" | "iv" => Ok(ViewKind::Interconnection),
        "BrowserView" | "bv" => Ok(ViewKind::Browser),
        "ActionFlowView"
        | "afv"
        | "StateTransitionView"
        | "stv"
        | "SequenceView"
        | "sv"
        | "GeometryView"
        | "gev"
        | "GridView"
        | "grv" => Err(DiagramError {
            kind: "UnsupportedView".to_string(),
            message: format!(
                "The '{view_type}' view is a recognized SysML v2 standard view but is not yet \
                 supported by this diagram renderer. Supported views: General View, \
                 Interconnection View, Browser View."
            ),
        }),
        _ => Err(DiagramError {
            kind: "UnknownView".to_string(),
            message: format!(
                "Unknown view '{view_type}'. Expected a SysML v2 standard view \
                 (General View, Interconnection View, or Browser View)."
            ),
        }),
    }
}

/// Whether a symbol is included as a node under the given view kind.
///
/// Mirrors the intent of the SysML v2 StandardViewDefinitions: General shows
/// everything, Interconnection shows structural usages + connectors,
/// Browser shows the membership hierarchy (packages, definitions, usages).
fn symbol_passes_view(kind: SymbolKind, view: ViewKind) -> bool {
    match view {
        ViewKind::General => true,
        ViewKind::Interconnection => matches!(
            kind,
            SymbolKind::PartUsage
                | SymbolKind::PortUsage
                | SymbolKind::InterfaceUsage
                | SymbolKind::ConnectionUsage
                | SymbolKind::FlowConnectionUsage
                | SymbolKind::AllocationUsage
        ),
        ViewKind::Browser => matches!(
            kind,
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
                | SymbolKind::AttributeUsage
                | SymbolKind::PortUsage
                | SymbolKind::ViewDefinition
                | SymbolKind::ViewpointDefinition
        ),
    }
}

impl LspServer {
    /// Get diagram data for the workspace or a specific file.
    /// Returns raw symbol data - presentation logic belongs in the frontend.
    pub fn get_diagram(&mut self, file_path: Option<&Path>, view_type: &str) -> DiagramData {
        // Resolve the requested view up front. An unsupported/unknown view is a
        // surfaceable error, not a reason to render a generic fallback graph.
        let view_kind = match resolve_view_kind(view_type) {
            Ok(k) => k,
            Err(error) => {
                return DiagramData {
                    symbols: Vec::new(),
                    relationships: Vec::new(),
                    view_type: view_type.to_string(),
                    error: Some(error),
                };
            }
        };

        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();

        // Candidate symbols: a specific file, or the whole workspace.
        let candidates: Vec<&HirSymbol> = if let Some(path) = file_path {
            let path_str = path.to_string_lossy();
            match analysis.get_file_id(&path_str) {
                Some(file_id) => index.symbols_in_file(file_id),
                None => {
                    return DiagramData {
                        symbols: Vec::new(),
                        relationships: Vec::new(),
                        view_type: view_type.to_string(),
                        error: Some(DiagramError {
                            kind: "NoFile".to_string(),
                            message: format!(
                                "No parsed document found for '{path_str}'. Open/save the file \
                                 before requesting its diagram."
                            ),
                        }),
                    };
                }
            }
        } else {
            index.all_symbols().collect()
        };

        // Build a parent -> feature-strings map from the FULL candidate set, so
        // a node's attributes/ports/parameters show as features even when the
        // active view would not surface them as standalone nodes.
        let mut features_by_parent: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for symbol in &candidates {
            if matches!(
                symbol.kind,
                SymbolKind::AttributeUsage | SymbolKind::PortUsage | SymbolKind::ReferenceUsage
            ) && let Some(parent) = extract_parent(&symbol.qualified_name)
            {
                let label = match symbol.supertypes.first() {
                    Some(ty) => format!("{}: {}", symbol.name, ty),
                    None => symbol.name.to_string(),
                };
                features_by_parent.entry(parent).or_default().push(label);
            }
        }

        let mut symbols = Vec::new();

        for symbol in &candidates {
            if !symbol_passes_view(symbol.kind, view_kind) {
                continue;
            }
            if let Some(mut diagram_symbol) = convert_symbol_to_diagram(symbol) {
                if let Some(feats) = features_by_parent.get(&diagram_symbol.qualified_name) {
                    diagram_symbol.features = Some(feats.clone());
                }
                symbols.push(diagram_symbol);
            }
        }

        // Edges are built only *after* the node set is known: React Flow silently
        // drops any edge whose source or target is not a rendered node, so an edge
        // to an off-canvas element (e.g. a type defined in another file) is not an
        // edge worth emitting. Resolve every relationship against these nodes.
        let node_names: std::collections::HashSet<&str> =
            symbols.iter().map(|s| s.qualified_name.as_str()).collect();
        // Index nodes by their simple (last-segment) name so a `typed_by` reference
        // like "Engine" can resolve to a rendered node "Pkg::Engine" when present.
        // `supertypes` are typically short names, not fully-qualified paths.
        let mut nodes_by_simple: std::collections::HashMap<&str, &str> =
            std::collections::HashMap::new();
        for s in &symbols {
            let simple = s
                .qualified_name
                .rsplit("::")
                .next()
                .unwrap_or(&s.qualified_name);
            nodes_by_simple
                .entry(simple)
                .or_insert(s.qualified_name.as_str());
        }

        let mut relationships = Vec::new();
        for s in &symbols {
            // Containment: parent -> child, emitted only when the parent is itself
            // a rendered node. These endpoints always exist, so the edge renders.
            if let Some(parent) = s.parent.as_deref()
                && node_names.contains(parent)
            {
                relationships.push(DiagramRelationship {
                    rel_type: "membership".to_string(),
                    source: parent.to_string(),
                    target: s.qualified_name.clone(),
                });
            }
            // Typing: usage -> its type, emitted only when the type resolves to a
            // rendered node. Match the exact qualified name first, then fall back to
            // the simple name; skip self-edges and unresolved (off-canvas) types.
            if let Some(typed_by) = s.typed_by.as_deref() {
                let target = if node_names.contains(typed_by) {
                    Some(typed_by.to_string())
                } else {
                    let simple = typed_by.rsplit("::").next().unwrap_or(typed_by);
                    nodes_by_simple.get(simple).map(|q| q.to_string())
                };
                if let Some(target) = target
                    && target != s.qualified_name
                {
                    relationships.push(DiagramRelationship {
                        rel_type: "typing".to_string(),
                        source: s.qualified_name.clone(),
                        target,
                    });
                }
            }
        }

        DiagramData {
            symbols,
            relationships,
            view_type: view_type.to_string(),
            error: None,
        }
    }
}

/// Convert a HirSymbol to DiagramSymbol
fn convert_symbol_to_diagram(symbol: &HirSymbol) -> Option<DiagramSymbol> {
    let name = symbol.name.to_string();
    let qualified_name = symbol.qualified_name.to_string();
    let parent = extract_parent(&qualified_name);
    let typed_by = symbol.supertypes.first().map(|s| s.to_string());

    // Determine high-level kind and specific sub-kind
    let (kind, definition_kind, usage_kind) = match symbol.kind {
        // Definitions
        SymbolKind::PartDefinition => ("Definition", Some("Part"), None),
        SymbolKind::ItemDefinition => ("Definition", Some("Item"), None),
        SymbolKind::ActionDefinition => ("Definition", Some("Action"), None),
        SymbolKind::PortDefinition => ("Definition", Some("Port"), None),
        SymbolKind::AttributeDefinition => ("Definition", Some("Attribute"), None),
        SymbolKind::ConnectionDefinition => ("Definition", Some("Connection"), None),
        SymbolKind::InterfaceDefinition => ("Definition", Some("Interface"), None),
        SymbolKind::AllocationDefinition => ("Definition", Some("Allocation"), None),
        SymbolKind::RequirementDefinition => ("Definition", Some("Requirement"), None),
        SymbolKind::ConstraintDefinition => ("Definition", Some("Constraint"), None),
        SymbolKind::StateDefinition => ("Definition", Some("State"), None),
        SymbolKind::CalculationDefinition => ("Definition", Some("Calculation"), None),
        SymbolKind::OccurrenceDefinition => ("Definition", Some("Occurrence"), None),
        SymbolKind::UseCaseDefinition => ("Definition", Some("UseCase"), None),
        SymbolKind::AnalysisCaseDefinition => ("Definition", Some("AnalysisCase"), None),
        SymbolKind::VerificationCaseDefinition => ("Definition", Some("VerificationCase"), None),
        SymbolKind::ConcernDefinition => ("Definition", Some("Concern"), None),
        SymbolKind::ViewDefinition => ("Definition", Some("View"), None),
        SymbolKind::ViewpointDefinition => ("Definition", Some("Viewpoint"), None),
        SymbolKind::RenderingDefinition => ("Definition", Some("Rendering"), None),
        SymbolKind::EnumerationDefinition => ("Definition", Some("Enumeration"), None),
        SymbolKind::MetadataDefinition => ("Definition", Some("Metadata"), None),
        SymbolKind::Interaction => ("Definition", Some("Interaction"), None),
        SymbolKind::DataType => ("Definition", Some("DataType"), None),
        SymbolKind::Class => ("Definition", Some("Class"), None),
        SymbolKind::Structure => ("Definition", Some("Structure"), None),
        SymbolKind::Behavior => ("Definition", Some("Behavior"), None),
        SymbolKind::Function => ("Definition", Some("Function"), None),
        SymbolKind::Association => ("Definition", Some("Association"), None),

        // Usages
        SymbolKind::PartUsage => ("Usage", None, Some("Part")),
        SymbolKind::ItemUsage => ("Usage", None, Some("Item")),
        SymbolKind::ActionUsage => ("Usage", None, Some("Action")),
        SymbolKind::PerformActionUsage => ("Usage", None, Some("PerformAction")),
        SymbolKind::PortUsage => ("Usage", None, Some("Port")),
        SymbolKind::AttributeUsage => ("Usage", None, Some("Attribute")),
        SymbolKind::ConnectionUsage => ("Usage", None, Some("Connection")),
        SymbolKind::InterfaceUsage => ("Usage", None, Some("Interface")),
        SymbolKind::AllocationUsage => ("Usage", None, Some("Allocation")),
        SymbolKind::RequirementUsage => ("Usage", None, Some("Requirement")),
        SymbolKind::SatisfyRequirementUsage => ("Usage", None, Some("SatisfyRequirement")),
        SymbolKind::ConstraintUsage => ("Usage", None, Some("Constraint")),
        SymbolKind::AssertConstraintUsage => ("Usage", None, Some("AssertConstraint")),
        SymbolKind::StateUsage => ("Usage", None, Some("State")),
        SymbolKind::ExhibitStateUsage => ("Usage", None, Some("ExhibitState")),
        SymbolKind::TransitionUsage => ("Usage", None, Some("Transition")),
        SymbolKind::CalculationUsage => ("Usage", None, Some("Calculation")),
        SymbolKind::ReferenceUsage => ("Usage", None, Some("Reference")),
        SymbolKind::OccurrenceUsage => ("Usage", None, Some("Occurrence")),
        SymbolKind::UseCaseUsage => ("Usage", None, Some("UseCase")),
        SymbolKind::IncludeUseCaseUsage => ("Usage", None, Some("IncludeUseCase")),
        SymbolKind::AnalysisCaseUsage => ("Usage", None, Some("AnalysisCase")),
        SymbolKind::VerificationCaseUsage => ("Usage", None, Some("VerificationCase")),
        SymbolKind::SuccessionUsage => ("Usage", None, Some("Succession")),
        SymbolKind::FlowConnectionUsage => ("Usage", None, Some("Flow")),
        SymbolKind::ViewUsage => ("Usage", None, Some("View")),
        SymbolKind::ViewpointUsage => ("Usage", None, Some("Viewpoint")),
        SymbolKind::RenderingUsage => ("Usage", None, Some("Rendering")),

        // Other
        SymbolKind::Package => ("Package", None, None),
        SymbolKind::ExposeRelationship
        | SymbolKind::Alias
        | SymbolKind::Import
        | SymbolKind::Comment
        | SymbolKind::Dependency
        | SymbolKind::Other => {
            return None;
        }
    };

    Some(DiagramSymbol {
        name,
        qualified_name,
        kind: kind.to_string(),
        definition_kind: definition_kind.map(String::from),
        usage_kind: usage_kind.map(String::from),
        parent,
        features: None,
        typed_by,
        direction: None,
    })
}

/// Extract parent qualified name from a fully qualified name.
/// e.g., "Package::SubPkg::Element" -> Some("Package::SubPkg")
///       "TopLevel" -> None (no parent)
fn extract_parent(qualified_name: &str) -> Option<String> {
    qualified_name
        .rfind("::")
        .map(|idx| qualified_name[..idx].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that DiagramSymbol serializes correctly with camelCase
    #[test]
    fn test_diagram_symbol_serialization() {
        let symbol = DiagramSymbol {
            name: "MyPart".to_string(),
            qualified_name: "Package::MyPart".to_string(),
            kind: "Definition".to_string(),
            definition_kind: Some("Part".to_string()),
            usage_kind: None,
            parent: Some("Package".to_string()),
            features: Some(vec!["feature1".to_string()]),
            typed_by: None,
            direction: None,
        };

        let json = serde_json::to_string(&symbol).unwrap();

        // Must use camelCase for JS consumption
        assert!(
            json.contains("\"qualifiedName\""),
            "Should use camelCase: {}",
            json
        );
        assert!(
            json.contains("\"definitionKind\""),
            "Should use camelCase: {}",
            json
        );
        assert!(
            json.contains("\"parent\""),
            "Should include parent: {}",
            json
        );
        assert!(
            !json.contains("\"qualified_name\""),
            "Should NOT use snake_case: {}",
            json
        );
        assert!(
            !json.contains("\"definition_kind\""),
            "Should NOT use snake_case: {}",
            json
        );
    }

    /// Test that DiagramData serializes correctly including view_type
    #[test]
    fn test_diagram_data_serialization() {
        let data = DiagramData {
            symbols: vec![DiagramSymbol {
                name: "Test".to_string(),
                qualified_name: "Pkg::Test".to_string(),
                kind: "Definition".to_string(),
                definition_kind: Some("Part".to_string()),
                usage_kind: None,
                parent: Some("Pkg".to_string()),
                features: None,
                typed_by: None,
                direction: None,
            }],
            relationships: vec![DiagramRelationship {
                rel_type: "typing".to_string(),
                source: "Pkg::A".to_string(),
                target: "Pkg::B".to_string(),
            }],
            view_type: "GeneralView".to_string(),
            error: None,
        };

        let json = serde_json::to_string(&data).unwrap();

        assert!(json.contains("\"symbols\""));
        assert!(json.contains("\"relationships\""));
        assert!(json.contains("\"type\":\"typing\"")); // rel_type serializes as "type"
        assert!(
            json.contains("\"viewType\":\"GeneralView\""),
            "Should include viewType in camelCase: {}",
            json
        );
    }

    #[test]
    fn test_resolve_view_kind_supported() {
        assert_eq!(resolve_view_kind("GeneralView").unwrap(), ViewKind::General);
        assert_eq!(resolve_view_kind("gv").unwrap(), ViewKind::General);
        assert_eq!(
            resolve_view_kind("InterconnectionView").unwrap(),
            ViewKind::Interconnection
        );
        assert_eq!(resolve_view_kind("iv").unwrap(), ViewKind::Interconnection);
        assert_eq!(resolve_view_kind("BrowserView").unwrap(), ViewKind::Browser);
        // Accepts qualified stdlib names too.
        assert_eq!(
            resolve_view_kind("StandardViewDefinitions::bv").unwrap(),
            ViewKind::Browser
        );
    }

    #[test]
    fn test_resolve_view_kind_unsupported_is_error_not_fallback() {
        // Known standard views we don't render yet must error, never fall back.
        for v in [
            "ActionFlowView",
            "afv",
            "SequenceView",
            "GridView",
            "GeometryView",
        ] {
            let err = resolve_view_kind(v).expect_err("should be an error");
            assert_eq!(err.kind, "UnsupportedView", "view {v}");
        }
    }

    #[test]
    fn test_resolve_view_kind_unknown_is_error() {
        let err = resolve_view_kind("BogusView").expect_err("should be an error");
        assert_eq!(err.kind, "UnknownView");
    }

    /// Test that GetDiagramParams deserializes with default view_type
    #[test]
    fn test_get_diagram_params_default_view_type() {
        let json = r#"{"uri": "file:///test.sysml"}"#;
        let params: GetDiagramParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.uri, Some("file:///test.sysml".to_string()));
        assert_eq!(params.view_type, "GeneralView");
    }

    /// Test that GetDiagramParams deserializes with explicit view_type
    #[test]
    fn test_get_diagram_params_explicit_view_type() {
        let json = r#"{"uri": "file:///test.sysml", "viewType": "InterconnectionView"}"#;
        let params: GetDiagramParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.uri, Some("file:///test.sysml".to_string()));
        assert_eq!(params.view_type, "InterconnectionView");
    }

    /// Test that GetDiagramParams works without uri (whole workspace)
    #[test]
    fn test_get_diagram_params_no_uri() {
        let json = r#"{}"#;
        let params: GetDiagramParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.uri, None);
        assert_eq!(params.view_type, "GeneralView");
    }

    /// Test convert_symbol_to_diagram for Definition symbols
    #[test]
    fn test_convert_definition_symbol() {
        use syster::base::FileId;

        let symbol = HirSymbol {
            name: "Vehicle".into(),
            short_name: None,
            qualified_name: "Pkg::Vehicle".into(),
            element_id: "test-id-1".into(),
            kind: SymbolKind::PartDefinition,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            doc: None,
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_composite: None,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "Vehicle");
        assert_eq!(diagram_symbol.qualified_name, "Pkg::Vehicle");
        assert_eq!(diagram_symbol.kind, "Definition");
        assert_eq!(diagram_symbol.definition_kind, Some("Part".to_string()));
        assert_eq!(diagram_symbol.usage_kind, None);
        assert_eq!(diagram_symbol.parent, Some("Pkg".to_string()));
        assert!(diagram_symbol.typed_by.is_none());
    }

    /// Test convert_symbol_to_diagram for Usage symbols
    #[test]
    fn test_convert_usage_symbol() {
        use syster::base::FileId;

        let symbol = HirSymbol {
            name: "engine".into(),
            short_name: None,
            qualified_name: "Pkg::Vehicle::engine".into(),
            element_id: "test-id-2".into(),
            kind: SymbolKind::PartUsage,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: vec!["Engine".into()],
            relationships: Vec::new(),
            doc: None,
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_composite: None,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "engine");
        assert_eq!(diagram_symbol.qualified_name, "Pkg::Vehicle::engine");
        assert_eq!(diagram_symbol.kind, "Usage");
        assert_eq!(diagram_symbol.definition_kind, None);
        assert_eq!(diagram_symbol.usage_kind, Some("Part".to_string()));
        assert_eq!(diagram_symbol.parent, Some("Pkg::Vehicle".to_string()));
        assert_eq!(diagram_symbol.typed_by, Some("Engine".to_string()));
    }

    /// Test convert_symbol_to_diagram for Package symbols
    #[test]
    fn test_convert_package_symbol() {
        use syster::base::FileId;

        let symbol = HirSymbol {
            name: "MyPackage".into(),
            short_name: None,
            qualified_name: "Root::MyPackage".into(),
            element_id: "test-id-3".into(),
            kind: SymbolKind::Package,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            doc: None,
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_composite: None,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "MyPackage");
        assert_eq!(diagram_symbol.qualified_name, "Root::MyPackage");
        assert_eq!(diagram_symbol.kind, "Package");
        assert_eq!(diagram_symbol.definition_kind, None);
        assert_eq!(diagram_symbol.usage_kind, None);
        assert_eq!(diagram_symbol.parent, Some("Root".to_string()));
    }

    /// Test that Alias symbols are skipped (return None)
    #[test]
    fn test_convert_alias_symbol_returns_none() {
        use syster::base::FileId;

        let symbol = HirSymbol {
            name: "MyAlias".into(),
            short_name: None,
            qualified_name: "Pkg::MyAlias".into(),
            element_id: "test-id-4".into(),
            kind: SymbolKind::Alias,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            doc: None,
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_composite: None,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        };

        assert!(convert_symbol_to_diagram(&symbol).is_none());
    }

    /// Test that Import symbols are skipped (return None)
    #[test]
    fn test_convert_import_symbol_returns_none() {
        use syster::base::FileId;

        let symbol = HirSymbol {
            name: "_import".into(),
            short_name: None,
            qualified_name: "Pkg::_import_Other::Thing".into(),
            element_id: "test-id-5".into(),
            kind: SymbolKind::Import,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            doc: None,
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_composite: None,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        };

        assert!(convert_symbol_to_diagram(&symbol).is_none());
    }

    /// Test that kind/definitionKind combine to form node types
    #[test]
    fn test_definition_kind_format() {
        // The viewer combines kind + definitionKind to form node types
        // kind="Definition", definitionKind="Part" => "PartDef"
        assert_eq!(format!("{}Def", "Part"), "PartDef");
        assert_eq!(format!("{}Def", "Port"), "PortDef");
        assert_eq!(format!("{}Def", "Action"), "ActionDef");
        assert_eq!(format!("{}Def", "Item"), "ItemDef");
    }

    #[test]
    fn test_usage_kind_format() {
        // The viewer combines kind + usageKind to form node types
        // kind="Usage", usageKind="Part" => "PartUsage"
        assert_eq!(format!("{}Usage", "Part"), "PartUsage");
        assert_eq!(format!("{}Usage", "Port"), "PortUsage");
        assert_eq!(format!("{}Usage", "Action"), "ActionUsage");
        assert_eq!(format!("{}Usage", "Item"), "ItemUsage");
    }

    #[test]
    fn test_extract_parent() {
        // Nested: extract parent
        assert_eq!(
            extract_parent("Package::SubPkg::Element"),
            Some("Package::SubPkg".to_string())
        );
        assert_eq!(
            extract_parent("Package::Element"),
            Some("Package".to_string())
        );

        // Top-level: no parent
        assert_eq!(extract_parent("TopLevel"), None);

        // Edge case: empty string
        assert_eq!(extract_parent(""), None);
    }
}
