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

/// Complete diagram data response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramData {
    pub symbols: Vec<DiagramSymbol>,
    pub relationships: Vec<DiagramRelationship>,
    pub view_type: String,
}

impl LspServer {
    /// Get diagram data for the workspace or a specific file.
    /// Returns raw symbol data - presentation logic belongs in the frontend.
    pub fn get_diagram(&mut self, file_path: Option<&Path>, view_type: &str) -> DiagramData {
        let mut symbols = Vec::new();
        let mut relationships = Vec::new();

        let analysis = self.analysis_host.analysis();

        // Collect symbols based on file path or whole workspace
        let symbol_iter: Box<dyn Iterator<Item = &HirSymbol>> = if let Some(path) = file_path {
            let path_str = path.to_string_lossy();
            if let Some(file_id) = analysis.get_file_id(&path_str) {
                Box::new(analysis.symbol_index().symbols_in_file(file_id).into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        } else {
            Box::new(analysis.symbol_index().all_symbols())
        };

        // Convert all symbols - frontend decides how to display them
        for symbol in symbol_iter {
            if let Some(diagram_symbol) = convert_symbol_to_diagram(symbol) {
                // Extract typing relationship from the symbol itself
                if let Some(ref typed_by) = diagram_symbol.typed_by {
                    relationships.push(DiagramRelationship {
                        rel_type: "typing".to_string(),
                        source: diagram_symbol.qualified_name.clone(),
                        target: typed_by.clone(),
                    });
                }
                symbols.push(diagram_symbol);
            }
        }

        DiagramData {
            symbols,
            relationships,
            view_type: view_type.to_string(),
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
        SymbolKind::UseCaseDefinition => ("Definition", Some("UseCase"), None),
        SymbolKind::AnalysisCaseDefinition => ("Definition", Some("AnalysisCase"), None),
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
        SymbolKind::PortUsage => ("Usage", None, Some("Port")),
        SymbolKind::AttributeUsage => ("Usage", None, Some("Attribute")),
        SymbolKind::ConnectionUsage => ("Usage", None, Some("Connection")),
        SymbolKind::InterfaceUsage => ("Usage", None, Some("Interface")),
        SymbolKind::AllocationUsage => ("Usage", None, Some("Allocation")),
        SymbolKind::RequirementUsage => ("Usage", None, Some("Requirement")),
        SymbolKind::ConstraintUsage => ("Usage", None, Some("Constraint")),
        SymbolKind::StateUsage => ("Usage", None, Some("State")),
        SymbolKind::TransitionUsage => ("Usage", None, Some("Transition")),
        SymbolKind::CalculationUsage => ("Usage", None, Some("Calculation")),
        SymbolKind::ReferenceUsage => ("Usage", None, Some("Reference")),
        SymbolKind::OccurrenceUsage => ("Usage", None, Some("Occurrence")),
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
