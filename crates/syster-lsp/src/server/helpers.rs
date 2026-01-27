use async_lsp::lsp_types::{Position, Range, SymbolKind, Url};
use percent_encoding::percent_decode_str;
use std::path::PathBuf;
use syster::hir::SymbolKind as HirSymbolKind;

/// Convert a URI to a PathBuf, returning None if the conversion fails
pub fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

/// Decode percent-encoded strings (e.g., "my%20file.txt" -> "my file.txt")
///
/// Used to display file names to users with proper formatting instead of URL encoding.
/// Handles invalid encoding gracefully by returning the original string.
pub fn decode_uri_component(s: &str) -> String {
    percent_decode_str(s)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

/// Convert a character offset in a line to UTF-16 code units
pub fn char_offset_to_utf16(line: &str, char_offset: usize) -> u32 {
    line.chars()
        .take(char_offset)
        .map(|c| c.len_utf16())
        .sum::<usize>() as u32
}

/// Convert character offset to byte offset within a line
pub fn char_offset_to_byte(line: &str, char_offset: usize) -> usize {
    line.chars().take(char_offset).map(|c| c.len_utf8()).sum()
}

/// Convert LSP Position to byte offset in text
///
/// Handles multi-line documents by calculating line offsets and character positions
/// Note: Treats position.character as character count (not strict UTF-16 code units)
pub fn position_to_byte_offset(text: &str, pos: Position) -> Result<usize, String> {
    let line_idx = pos.line as usize;
    let char_offset = pos.character as usize;

    // Split by \n to handle both LF and CRLF (since \r\n split on \n leaves \r at line end)
    let lines: Vec<&str> = text.split('\n').collect();

    if line_idx > lines.len() {
        return Err(format!(
            "Line {} out of bounds (total lines: {})",
            line_idx,
            lines.len()
        ));
    }

    if line_idx == lines.len() {
        return Ok(text.len());
    }

    // Calculate byte offset up to the start of the target line
    let mut byte_offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i == line_idx {
            break;
        }
        byte_offset += line.len() + 1; // +1 for newline
    }

    // Add character offset within the line converted to bytes
    let line = lines[line_idx];
    let line_byte_offset = char_offset_to_byte(line, char_offset);

    Ok(byte_offset + line_byte_offset)
}

/// Apply a text edit to a string based on LSP range
pub fn apply_text_edit(text: &str, range: &Range, new_text: &str) -> Result<String, String> {
    let start_byte = position_to_byte_offset(text, range.start)?;
    let end_byte = position_to_byte_offset(text, range.end)?;

    if start_byte > end_byte {
        return Err(format!(
            "Invalid range: start ({start_byte}) > end ({end_byte})"
        ));
    }

    if end_byte > text.len() {
        return Err(format!(
            "Range end ({}) exceeds text length ({})",
            end_byte,
            text.len()
        ));
    }

    let mut result = String::with_capacity(text.len() + new_text.len());
    result.push_str(&text[..start_byte]);
    result.push_str(new_text);
    result.push_str(&text[end_byte..]);

    Ok(result)
}

/// Convert our Position to LSP Position
pub fn position_to_lsp_position(pos: &syster::core::Position) -> Position {
    Position {
        line: pos.line as u32,
        character: pos.column as u32,
    }
}

/// Convert HIR SymbolKind to LSP SymbolKind.
///
/// This is the single source of truth for symbol kind conversion in the LSP.
/// Used by document symbols, workspace symbols, and other features.
pub fn hir_to_lsp_symbol_kind(kind: HirSymbolKind) -> SymbolKind {
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
        | HirSymbolKind::EnumerationDef
        | HirSymbolKind::MetaclassDef
        | HirSymbolKind::InteractionDef => SymbolKind::CLASS,

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

/// Convert HIR SymbolKind to diagram node type string.
///
/// Returns None for kinds that shouldn't be shown in diagrams.
/// The returned strings must match NODE_TYPES in diagram-core.
pub fn hir_to_diagram_node_type(kind: HirSymbolKind) -> Option<&'static str> {
    match kind {
        // Definitions
        HirSymbolKind::PartDef => Some("PartDef"),
        HirSymbolKind::ItemDef => Some("ItemDef"),
        HirSymbolKind::ActionDef => Some("ActionDef"),
        HirSymbolKind::PortDef => Some("PortDef"),
        HirSymbolKind::AttributeDef => Some("AttributeDef"),
        HirSymbolKind::ConnectionDef => Some("ConnectionDef"),
        HirSymbolKind::InterfaceDef => Some("InterfaceDef"),
        HirSymbolKind::AllocationDef => Some("AllocationDef"),
        HirSymbolKind::RequirementDef => Some("RequirementDef"),
        HirSymbolKind::ConstraintDef => Some("ConstraintDef"),
        HirSymbolKind::StateDef => Some("StateDef"),
        HirSymbolKind::CalculationDef => Some("CalculationDef"),
        HirSymbolKind::UseCaseDef => Some("UseCaseDef"),
        HirSymbolKind::AnalysisCaseDef => Some("AnalysisCaseDef"),
        HirSymbolKind::ConcernDef => Some("ConcernDef"),
        HirSymbolKind::ViewDef => Some("ViewDef"),
        HirSymbolKind::ViewpointDef => Some("ViewpointDef"),
        HirSymbolKind::RenderingDef => Some("RenderingDef"),
        HirSymbolKind::EnumerationDef => Some("EnumerationDef"),
        HirSymbolKind::MetaclassDef => Some("MetaclassDef"),
        HirSymbolKind::InteractionDef => Some("InteractionDef"),

        // Usages
        HirSymbolKind::PartUsage => Some("PartUsage"),
        HirSymbolKind::ItemUsage => Some("ItemUsage"),
        HirSymbolKind::ActionUsage => Some("ActionUsage"),
        HirSymbolKind::PortUsage => Some("PortUsage"),
        HirSymbolKind::AttributeUsage => Some("AttributeUsage"),
        HirSymbolKind::ConnectionUsage => Some("ConnectionUsage"),
        HirSymbolKind::InterfaceUsage => Some("InterfaceUsage"),
        HirSymbolKind::AllocationUsage => Some("AllocationUsage"),
        HirSymbolKind::RequirementUsage => Some("RequirementUsage"),
        HirSymbolKind::ConstraintUsage => Some("ConstraintUsage"),
        HirSymbolKind::StateUsage => Some("StateUsage"),
        HirSymbolKind::CalculationUsage => Some("CalculationUsage"),
        HirSymbolKind::ReferenceUsage => Some("ReferenceUsage"),
        HirSymbolKind::OccurrenceUsage => Some("OccurrenceUsage"),
        HirSymbolKind::FlowUsage => Some("FlowUsage"),

        // Package
        HirSymbolKind::Package => Some("Package"),

        // Not shown in diagrams
        HirSymbolKind::Alias
        | HirSymbolKind::Import
        | HirSymbolKind::Comment
        | HirSymbolKind::Dependency
        | HirSymbolKind::Other => None,
    }
}
