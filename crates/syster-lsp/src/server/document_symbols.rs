use super::LspServer;
use async_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
use std::collections::HashMap;
use std::path::Path;
use syster::hir::SymbolKind as HirSymbolKind;

impl LspServer {
    /// Get all symbols in a document for the outline view.
    ///
    /// Uses the new HIR-based IDE layer.
    pub fn get_document_symbols(&mut self, file_path: &Path) -> Vec<DocumentSymbol> {
        let path_str = file_path.to_string_lossy();
        let analysis = self.analysis_host.analysis();

        let file_id = match analysis.get_file_id(&path_str) {
            Some(id) => id,
            None => return Vec::new(),
        };

        // Use the Analysis document_symbols method
        let symbols = analysis.document_symbols(file_id);

        let flat_symbols: Vec<(String, DocumentSymbol)> = symbols
            .into_iter()
            .map(|sym| {
                let range = Range {
                    start: Position {
                        line: sym.start_line,
                        character: sym.start_col,
                    },
                    end: Position {
                        line: sym.end_line,
                        character: sym.end_col,
                    },
                };

                let doc_symbol = DocumentSymbol {
                    name: sym.name.to_string(),
                    detail: Some(sym.qualified_name.to_string()),
                    kind: convert_symbol_kind(sym.kind),
                    range,
                    selection_range: range,
                    children: Some(Vec::new()),
                    tags: None,
                    #[allow(deprecated)]
                    deprecated: None,
                };

                (sym.qualified_name.to_string(), doc_symbol)
            })
            .collect();

        // Build hierarchy from qualified names
        self.build_symbol_hierarchy(flat_symbols)
    }

    /// Build a hierarchical structure from flat symbols using qualified names
    fn build_symbol_hierarchy(
        &self,
        flat_symbols: Vec<(String, DocumentSymbol)>,
    ) -> Vec<DocumentSymbol> {
        let mut symbol_map: HashMap<String, DocumentSymbol> = HashMap::new();

        // First, add all symbols to the map
        for (qualified_name, symbol) in flat_symbols {
            symbol_map.insert(qualified_name, symbol);
        }

        // Get all names and sort by depth (MORE "::" first, so deepest children are processed first)
        let mut all_names: Vec<String> = symbol_map.keys().cloned().collect();
        all_names.sort_by(|a: &String, b: &String| {
            let depth_a = a.matches("::").count();
            let depth_b = b.matches("::").count();
            depth_b.cmp(&depth_a) // Reverse order: deepest first
        });

        // Build hierarchy by moving children into parents, starting from deepest
        for qualified_name in &all_names {
            if let Some(last_separator) = qualified_name.rfind("::") {
                let parent_name = &qualified_name[..last_separator];

                // Check if parent exists and child hasn't been moved yet
                if symbol_map.contains_key(parent_name) && symbol_map.contains_key(qualified_name) {
                    // Remove child from map
                    let child = symbol_map.remove(qualified_name).unwrap();

                    // Add child to parent's children
                    if let Some(parent) = symbol_map.get_mut(parent_name)
                        && let Some(ref mut children) = parent.children
                    {
                        children.push(child);
                    }
                }
            }
        }

        // Remaining symbols in the map are root symbols
        let mut root_symbols: Vec<DocumentSymbol> = symbol_map.into_values().collect();
        root_symbols.sort_by(|a, b| a.name.cmp(&b.name));
        root_symbols
    }
}

fn convert_symbol_kind(kind: HirSymbolKind) -> SymbolKind {
    match kind {
        HirSymbolKind::Package => SymbolKind::NAMESPACE,

        // Definitions are classes
        HirSymbolKind::PartDef
        | HirSymbolKind::ItemDef
        | HirSymbolKind::ActionDef
        | HirSymbolKind::PortDef
        | HirSymbolKind::AttributeDef
        | HirSymbolKind::ConnectionDef
        | HirSymbolKind::InterfaceDef
        | HirSymbolKind::AllocationDef
        | HirSymbolKind::RequirementDef
        | HirSymbolKind::ConstraintDef
        | HirSymbolKind::StateDef
        | HirSymbolKind::CalculationDef
        | HirSymbolKind::UseCaseDef
        | HirSymbolKind::AnalysisCaseDef
        | HirSymbolKind::ConcernDef
        | HirSymbolKind::ViewDef
        | HirSymbolKind::ViewpointDef
        | HirSymbolKind::RenderingDef
        | HirSymbolKind::EnumerationDef => SymbolKind::CLASS,

        // Usages are properties
        HirSymbolKind::PartUsage
        | HirSymbolKind::ItemUsage
        | HirSymbolKind::ActionUsage
        | HirSymbolKind::PortUsage
        | HirSymbolKind::AttributeUsage
        | HirSymbolKind::ConnectionUsage
        | HirSymbolKind::InterfaceUsage
        | HirSymbolKind::AllocationUsage
        | HirSymbolKind::RequirementUsage
        | HirSymbolKind::ConstraintUsage
        | HirSymbolKind::StateUsage
        | HirSymbolKind::CalculationUsage
        | HirSymbolKind::ReferenceUsage
        | HirSymbolKind::OccurrenceUsage
        | HirSymbolKind::FlowUsage => SymbolKind::PROPERTY,

        HirSymbolKind::Alias => SymbolKind::VARIABLE,
        HirSymbolKind::Import => SymbolKind::NAMESPACE,
        HirSymbolKind::Comment => SymbolKind::STRING,
        HirSymbolKind::Dependency => SymbolKind::VARIABLE,
        HirSymbolKind::Other => SymbolKind::VARIABLE,
    }
}
