use super::LspServer;
use async_lsp::lsp_types::{Location, OneOf, Position, Range, SymbolKind, Url, WorkspaceSymbol};
use syster::hir::SymbolKind as HirSymbolKind;

impl LspServer {
    /// Get workspace-wide symbols filtered by the user's query.
    ///
    /// Uses the new HIR-based IDE layer.
    pub fn get_workspace_symbols(&mut self, query: &str) -> Vec<WorkspaceSymbol> {
        if self.ensure_workspace_loaded().is_err() {
            return Vec::new();
        }

        let query = query.trim();
        let query_opt = if query.is_empty() { None } else { Some(query) };

        let analysis = self.analysis_host.analysis();

        // Use the Analysis workspace_symbols method
        let symbols = analysis.workspace_symbols(query_opt);

        symbols
            .into_iter()
            .filter_map(|sym| {
                let path = analysis.get_file_path(sym.file)?;
                let uri = Url::from_file_path(path).ok()?;

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

                Some(WorkspaceSymbol {
                    name: sym.name.to_string(),
                    kind: convert_symbol_kind(sym.kind),
                    tags: None,
                    location: OneOf::Left(Location { uri, range }),
                    container_name: sym.container_name().map(|s| s.to_string()),
                    data: None,
                })
            })
            .collect()
    }
}

fn convert_symbol_kind(kind: HirSymbolKind) -> SymbolKind {
    match kind {
        HirSymbolKind::Package => SymbolKind::NAMESPACE,
        
        // Definitions are classes
        HirSymbolKind::PartDef | HirSymbolKind::ItemDef | HirSymbolKind::ActionDef |
        HirSymbolKind::PortDef | HirSymbolKind::AttributeDef | HirSymbolKind::ConnectionDef |
        HirSymbolKind::InterfaceDef | HirSymbolKind::AllocationDef | HirSymbolKind::RequirementDef |
        HirSymbolKind::ConstraintDef | HirSymbolKind::StateDef | HirSymbolKind::CalculationDef |
        HirSymbolKind::UseCaseDef | HirSymbolKind::AnalysisCaseDef | HirSymbolKind::ConcernDef |
        HirSymbolKind::ViewDef | HirSymbolKind::ViewpointDef | HirSymbolKind::RenderingDef |
        HirSymbolKind::EnumerationDef => SymbolKind::CLASS,
        
        // Usages are properties
        HirSymbolKind::PartUsage | HirSymbolKind::ItemUsage | HirSymbolKind::ActionUsage |
        HirSymbolKind::PortUsage | HirSymbolKind::AttributeUsage | HirSymbolKind::ConnectionUsage |
        HirSymbolKind::InterfaceUsage | HirSymbolKind::AllocationUsage | HirSymbolKind::RequirementUsage |
        HirSymbolKind::ConstraintUsage | HirSymbolKind::StateUsage | HirSymbolKind::CalculationUsage |
        HirSymbolKind::ReferenceUsage | HirSymbolKind::OccurrenceUsage | HirSymbolKind::FlowUsage => {
            SymbolKind::PROPERTY
        }
        
        HirSymbolKind::Alias => SymbolKind::VARIABLE,
        HirSymbolKind::Import => SymbolKind::NAMESPACE,
        HirSymbolKind::Comment => SymbolKind::STRING,
        HirSymbolKind::Dependency => SymbolKind::VARIABLE,
        HirSymbolKind::Other => SymbolKind::VARIABLE,
    }
}
