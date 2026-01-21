//! Diagram data provider for VS Code webview integration.
//!
//! Provides diagram data (symbols + relationships) in a format consumable
//! by the diagram-core TypeScript package.
//!
//! IMPORTANT: The `node_type` field MUST match the NODE_TYPES values in
//! packages/diagram-core/src/sysml-nodes.ts. If they don't match, nodes
//! won't render in the diagram.

use super::LspServer;
use async_lsp::lsp_types::request::Request;
use serde::{Deserialize, Serialize};
use std::path::Path;
use syster::semantic::symbol_table::Symbol;

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
/// The `node_type` field MUST match NODE_TYPES from diagram-core.
/// Examples: "PartDef", "ItemUsage", "PortDef", "AttributeUsage"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramSymbol {
    /// Simple name of the element
    pub name: String,

    /// Fully qualified name (e.g., "Package::Element")
    pub qualified_name: String,

    /// Node type for rendering - MUST match NODE_TYPES values.
    /// Format: "{Kind}Def" for definitions, "{Kind}Usage" for usages.
    /// Examples: "PartDef", "ItemUsage", "PortDef"
    pub node_type: String,

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
    pub fn get_diagram(&self, file_path: Option<&Path>, view_type: &str) -> DiagramData {
        let mut symbols = Vec::new();
        let mut relationships = Vec::new();

        // Collect symbols based on file path or whole workspace
        let symbol_iter: Box<dyn Iterator<Item = &Symbol>> = if let Some(path) = file_path {
            let path_str = path.to_str().unwrap_or("");
            Box::new(
                self.workspace
                    .symbol_table()
                    .get_symbols_for_file(path_str)
                    .into_iter(),
            )
        } else {
            Box::new(self.workspace.symbol_table().iter_symbols())
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

/// Convert a Symbol to DiagramSymbol
fn convert_symbol_to_diagram(symbol: &Symbol) -> Option<DiagramSymbol> {
    match symbol {
        Symbol::Definition {
            name,
            qualified_name,
            kind,
            ..
        } => Some(DiagramSymbol {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            // Format: "{Kind}Def" - e.g., "Part" -> "PartDef"
            node_type: format!("{}Def", kind),
            parent: extract_parent(qualified_name),
            features: None,
            typed_by: None,
            direction: None,
        }),
        Symbol::Usage {
            name,
            qualified_name,
            kind,
            usage_type,
            ..
        } => Some(DiagramSymbol {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            // Format: "{Kind}Usage" - e.g., "Item" -> "ItemUsage"
            node_type: format!("{}Usage", kind),
            parent: extract_parent(qualified_name),
            features: None,
            typed_by: usage_type.clone(),
            direction: None,
        }),
        Symbol::Package {
            name,
            qualified_name,
            ..
        } => Some(DiagramSymbol {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            // Packages don't have a node type in diagram-ui yet
            // TODO: Add PackageDef to NODE_TYPES if we want to render packages
            node_type: "Package".to_string(),
            parent: extract_parent(qualified_name),
            features: None,
            typed_by: None,
            direction: None,
        }),
        Symbol::Feature {
            name,
            qualified_name,
            feature_type,
            ..
        } => Some(DiagramSymbol {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            // Features are typically rendered as part of their parent
            node_type: "Feature".to_string(),
            parent: extract_parent(qualified_name),
            features: None,
            typed_by: feature_type.clone(),
            direction: None,
        }),
        Symbol::Classifier {
            name,
            qualified_name,
            kind,
            ..
        } => Some(DiagramSymbol {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            // KerML classifiers - try to map to closest SysML equivalent
            node_type: format!("{}Def", kind),
            parent: extract_parent(qualified_name),
            features: None,
            typed_by: None,
            direction: None,
        }),
        // Skip Alias, Import, and Comment - not useful for diagrams
        Symbol::Alias { .. } | Symbol::Import { .. } | Symbol::Comment { .. } => None,
    }
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
            node_type: "PartDef".to_string(),
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
            json.contains("\"nodeType\""),
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
            !json.contains("\"node_type\""),
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
                node_type: "PartDef".to_string(),
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
        use syster::core::Span;

        let symbol = Symbol::Definition {
            name: "Vehicle".to_string(),
            qualified_name: "Pkg::Vehicle".to_string(),
            kind: "Part".to_string(),
            semantic_role: None,
            scope_id: 0,
            source_file: Some("test.sysml".to_string()),
            span: Some(Span::from_coords(0, 0, 0, 10)),
            documentation: None,
            specializes: Vec::new(),
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "Vehicle");
        assert_eq!(diagram_symbol.qualified_name, "Pkg::Vehicle");
        assert_eq!(diagram_symbol.node_type, "PartDef");
        assert_eq!(diagram_symbol.parent, Some("Pkg".to_string()));
        assert!(diagram_symbol.typed_by.is_none());
    }

    /// Test convert_symbol_to_diagram for Usage symbols
    #[test]
    fn test_convert_usage_symbol() {
        use syster::core::Span;

        let symbol = Symbol::Usage {
            name: "engine".to_string(),
            qualified_name: "Pkg::Vehicle::engine".to_string(),
            kind: "Part".to_string(),
            semantic_role: None,
            usage_type: Some("Engine".to_string()),
            scope_id: 0,
            source_file: Some("test.sysml".to_string()),
            span: Some(Span::from_coords(0, 0, 0, 10)),
            documentation: None,
            redefines: Vec::new(),
            performs: Vec::new(),
            references: Vec::new(),
            subsets: Vec::new(),
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "engine");
        assert_eq!(diagram_symbol.qualified_name, "Pkg::Vehicle::engine");
        assert_eq!(diagram_symbol.node_type, "PartUsage");
        assert_eq!(diagram_symbol.parent, Some("Pkg::Vehicle".to_string()));
        assert_eq!(diagram_symbol.typed_by, Some("Engine".to_string()));
    }

    /// Test convert_symbol_to_diagram for Package symbols
    #[test]
    fn test_convert_package_symbol() {
        use syster::core::Span;

        let symbol = Symbol::Package {
            name: "MyPackage".to_string(),
            qualified_name: "Root::MyPackage".to_string(),
            scope_id: 0,
            source_file: Some("test.sysml".to_string()),
            span: Some(Span::from_coords(0, 0, 0, 10)),
            documentation: None,
        };

        let diagram_symbol = convert_symbol_to_diagram(&symbol).unwrap();

        assert_eq!(diagram_symbol.name, "MyPackage");
        assert_eq!(diagram_symbol.qualified_name, "Root::MyPackage");
        assert_eq!(diagram_symbol.node_type, "Package");
        assert_eq!(diagram_symbol.parent, Some("Root".to_string()));
    }

    /// Test that Alias symbols are skipped (return None)
    #[test]
    fn test_convert_alias_symbol_returns_none() {
        use syster::core::Span;

        let symbol = Symbol::Alias {
            name: "MyAlias".to_string(),
            qualified_name: "Pkg::MyAlias".to_string(),
            target: "Target".to_string(),
            target_span: None,
            scope_id: 0,
            source_file: Some("test.sysml".to_string()),
            span: Some(Span::from_coords(0, 0, 0, 10)),
        };

        assert!(convert_symbol_to_diagram(&symbol).is_none());
    }

    /// Test that Import symbols are skipped (return None)
    #[test]
    fn test_convert_import_symbol_returns_none() {
        use syster::core::Span;

        let symbol = Symbol::Import {
            path: "Other::Thing".to_string(),
            path_span: None,
            qualified_name: "Pkg::_import_Other::Thing".to_string(),
            is_recursive: false,
            scope_id: 0,
            source_file: Some("test.sysml".to_string()),
            span: Some(Span::from_coords(0, 0, 0, 10)),
        };

        assert!(convert_symbol_to_diagram(&symbol).is_none());
    }

    /// Test node_type values match what diagram-ui expects
    #[test]
    fn test_node_type_format_for_definitions() {
        // These are how we format definition kinds
        assert_eq!(format!("{}Def", "Part"), "PartDef");
        assert_eq!(format!("{}Def", "Port"), "PortDef");
        assert_eq!(format!("{}Def", "Action"), "ActionDef");
        assert_eq!(format!("{}Def", "Item"), "ItemDef");
    }

    #[test]
    fn test_node_type_format_for_usages() {
        // These are how we format usage kinds
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
