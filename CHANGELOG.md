# Changelog

All notable changes to syster-lsp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1-alpha] - 2026-01-24

### Added

- **Type Definition Navigation** (`textDocument/typeDefinition`):
  - Navigate from usages directly to their type definitions
  - Works for typed features, parameters, and attributes
  - Standard LSP feature for "Go to Type Definition" command

- **Clickable Relationships in Hover**:
  - Relationships now render as markdown links in hover popups
  - Click to navigate directly to specialization targets, type definitions, etc.
  - Uses `MarkupContent` with `MarkupKind::Markdown` for proper link rendering

- **Range Formatting** (`textDocument/rangeFormatting`):
  - Format selected code ranges instead of entire documents
  - Supports standard LSP range formatting requests

### Changed

- **Hover Content Type**: Changed from `MarkedString::String` to `MarkupContent` with `MarkupKind::Markdown` to enable clickable links
- **syster-base**: Updated to v0.2.1-alpha with `ResolvedRelationship` and `goto_type_definition` support

## [0.2.0-alpha] - 2026-01-23

### 🚀 Major Update — Salsa-powered Incremental Analysis

This release integrates with syster-base 0.2.0-alpha's complete architectural rewrite, bringing incremental computation to the LSP server.

### Added

- **Semantic Diagnostics**: Full integration with syster-base's new `SemanticChecker`
  - Parse errors reported via `syster-parse` source
  - Semantic errors reported via `syster-semantic` source
  - Error codes: E0001 (undefined reference), E0002 (ambiguous), E0003 (type mismatch), E0004 (duplicate definition)
  - Warning codes: W0001 (unused symbol), W0002 (deprecated), W0003 (naming convention)
  - Related information linking to other source locations

- **AnalysisHost**: New unified analysis coordinator
  - Manages workspace, symbol index, and file maps
  - Integrates with Salsa for incremental computation
  - Efficient file ID mapping for fast lookups

### Changed

- **Incremental Updates**: File changes only recompute affected queries
  - Parsing memoized per-file via Salsa
  - Symbol extraction memoized per-file
  - Resolution cached in visibility maps

- **Architecture**: Migrated to syster-base 0.2.0-alpha
  - Uses `FileId` (4 bytes) instead of `PathBuf` throughout
  - Uses `SymbolIndex` for workspace-wide name resolution
  - Uses `Resolver` with scope-aware import handling

### Inherits from syster-base 0.2.0-alpha

All improvements from the base library are automatically available:
- Salsa-based incremental queries (`RootDatabase`, `FileText`, `parse_file`, etc.)
- Foundation types (`FileId`, `Name`, `Interner`, `TextRange`)
- Semantic IDs (`DefId`, `LocalDefId`)
- Implicit supertypes for all SysML definition kinds
- Anonymous scope naming with unique qualified names
- Invocation expression reference extraction
- Scope-aware import link resolution

## [0.1.13-alpha] - 2025-01-30

### Added

- Initial Language Server Protocol implementation
- Document symbols, diagnostics, hover, go-to-definition
- Document links for imports and type references
- Find references support
