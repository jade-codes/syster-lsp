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
        HirSymbolKind::PartDefinition
        | HirSymbolKind::ItemDefinition
        | HirSymbolKind::ActionDefinition
        | HirSymbolKind::PortDefinition
        | HirSymbolKind::AttributeDefinition
        | HirSymbolKind::ConnectionDefinition
        | HirSymbolKind::InterfaceDefinition
        | HirSymbolKind::AllocationDefinition
        | HirSymbolKind::RequirementDefinition
        | HirSymbolKind::ConstraintDefinition
        | HirSymbolKind::StateDefinition
        | HirSymbolKind::CalculationDefinition
        | HirSymbolKind::UseCaseDefinition
        | HirSymbolKind::AnalysisCaseDefinition
        | HirSymbolKind::ConcernDefinition
        | HirSymbolKind::ViewDefinition
        | HirSymbolKind::ViewpointDefinition
        | HirSymbolKind::RenderingDefinition
        | HirSymbolKind::EnumerationDefinition
        | HirSymbolKind::MetadataDefinition
        | HirSymbolKind::Interaction
        | HirSymbolKind::DataType
        | HirSymbolKind::Class
        | HirSymbolKind::Structure
        | HirSymbolKind::Behavior
        | HirSymbolKind::Function
        | HirSymbolKind::Association => SymbolKind::CLASS,

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
        | HirSymbolKind::TransitionUsage
        | HirSymbolKind::CalculationUsage
        | HirSymbolKind::ReferenceUsage
        | HirSymbolKind::OccurrenceUsage
        | HirSymbolKind::FlowConnectionUsage
        | HirSymbolKind::ViewUsage
        | HirSymbolKind::ViewpointUsage
        | HirSymbolKind::RenderingUsage => SymbolKind::PROPERTY,

        HirSymbolKind::ExposeRelationship => SymbolKind::VARIABLE,
        HirSymbolKind::Alias => SymbolKind::VARIABLE,
        HirSymbolKind::Import => SymbolKind::NAMESPACE,
        HirSymbolKind::Comment => SymbolKind::STRING,
        HirSymbolKind::Dependency => SymbolKind::VARIABLE,
        HirSymbolKind::Other => SymbolKind::VARIABLE,
    }
}
