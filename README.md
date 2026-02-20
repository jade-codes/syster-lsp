# Syster LSP

Language Server Protocol implementation for SysML v2 and KerML.

## Architecture

Built on [tower-lsp](https://github.com/ebkalderon/tower-lsp) and [syster-base](../base). The server maintains a live `AnalysisHost` that updates incrementally as files change.

```
┌─────────────────────────────────────────────────────────────────────┐
│                         LSP Server (tower-lsp)                      │
│                                                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌─────────┐ │
│  │completion│ │ goto-def │ │  hover   │ │references │ │ rename  │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └─────┬─────┘ └────┬────┘ │
│       │             │            │              │            │      │
│       └─────────────┴────────────┴──────────────┴────────────┘      │
│                                  │                                  │
│                                  ▼                                  │
│                     ┌────────────────────────┐                      │
│                     │     AnalysisHost        │                     │
│                     │  .set_file_content()    │                     │
│                     │  .analysis() → snapshot │                     │
│                     └────────────┬────────────┘                     │
│                                  │                                  │
│                                  ▼                                  │
│                     ┌────────────────────────┐                      │
│                     │  Salsa RootDatabase     │                     │
│                     │  (incremental queries)  │                     │
│                     └────────────────────────┘                      │
│                                                                     │
│  ┌──────────────────┐  ┌──────────────────┐                        │
│  │ background_tasks │  │  interchange     │  (feature-gated)       │
│  │ • diagnostics    │  │  • export/import │                        │
│  │ • indexing       │  │  • diagram data  │                        │
│  └──────────────────┘  └──────────────────┘                        │
└─────────────────────────────────────────────────────────────────────┘
          ↕ stdio / JSON-RPC
┌─────────────────────────────────────────────────────────────────────┐
│  VS Code (language-client extension)                                │
└─────────────────────────────────────────────────────────────────────┘
```

**Design:**
- Each LSP request calls the corresponding function in syster-base's `ide` module
- `AnalysisHost` is the mutable owner; `.analysis()` returns a read-only snapshot for concurrent reads
- Salsa ensures only affected queries recompute when a file changes
- Background tasks handle diagnostics publishing and workspace indexing

## Server Modules

| Module | LSP Feature(s) |
|--------|----------------|
| `completion.rs` | `textDocument/completion` — context-aware completions |
| `definition.rs` | `textDocument/definition` — jump to symbol definition |
| `type_definition.rs` | `textDocument/typeDefinition` — jump to type |
| `hover.rs` | `textDocument/hover` — type info, docs, qualified names |
| `references.rs` | `textDocument/references` — find all usages |
| `rename.rs` | `textDocument/rename` — rename symbols across workspace |
| `document_symbols.rs` | `textDocument/documentSymbol` — hierarchical outline |
| `workspace_symbols.rs` | `workspace/symbol` — search symbols across files |
| `semantic_tokens.rs` | `textDocument/semanticTokens` — rich syntax highlighting |
| `inlay_hints.rs` | `textDocument/inlayHint` — inline type annotations |
| `folding_ranges.rs` | `textDocument/foldingRange` — collapsible regions |
| `selection_range.rs` | `textDocument/selectionRange` — expand/shrink selection |
| `document_links.rs` | `textDocument/documentLink` — clickable imports/refs |
| `formatting.rs` | `textDocument/formatting` — auto-format SysML/KerML |
| `diagnostics.rs` | `textDocument/publishDiagnostics` — errors + warnings |
| `code_lens.rs` | `textDocument/codeLens` — inline reference counts |
| `diagram.rs` | Custom — diagram data for modeller/viewer extensions |
| `interchange.rs` | Custom — export/import commands (feature-gated) |
| `views.rs` | Custom — element view data for extensions |

## Building

```bash
# Debug build
cargo build -p syster-lsp

# Release build
cargo build --release -p syster-lsp

# With interchange support
cargo build --release -p syster-lsp --features interchange

# Run tests
cargo test -p syster-lsp

# Run clippy
cargo clippy -p syster-lsp -- -D warnings
```

## Usage

The server binary communicates over stdio using JSON-RPC. Any LSP-compatible editor can use it.

For VS Code, install the [language-client](../language-client) extension which spawns the server automatically.

```bash
# Manual launch (for other editors)
syster-lsp --stdio
```

## License

MIT
