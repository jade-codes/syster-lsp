# Syster LSP

Language Server Protocol implementation for SysML v2 and KerML.

## Architecture

Built on top of [syster-base](../syster-base), the LSP server uses **Salsa-based incremental computation** for efficient editing:

```
┌─────────────────────────────────────────────────────────────┐
│                      LSP Server                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Hover     │    │ Go-to-Def   │    │ Diagnostics │     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘     │
│         │                  │                   │            │
│         └──────────────────┼───────────────────┘            │
│                            ▼                                │
│                    ┌───────────────┐                        │
│                    │ AnalysisHost  │                        │
│                    │ (SymbolIndex) │                        │
│                    └───────┬───────┘                        │
│                            │                                │
│                            ▼                                │
│                    ┌───────────────┐                        │
│                    │  Salsa DB     │  ← Incremental queries │
│                    │ (RootDatabase)│                        │
│                    └───────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

**Key benefits:**
- **Incremental**: Only changed files are re-parsed
- **Memoized**: Queries cached automatically
- **Fast**: `FileId` (4 bytes) enables O(1) file lookups

## Components

- `crates/syster-lsp` - Rust LSP server binary

## Features

| Feature | Description |
|---------|-------------|
| Syntax highlighting | Semantic tokens for SysML/KerML keywords, types, etc. |
| Code completion | Context-aware completions for definitions, usages, imports |
| Go to definition | Jump to symbol definitions |
| Find references | Find all usages of a symbol |
| Hover documentation | Type info, documentation, qualified names |
| Document outline | Hierarchical symbol tree |
| Code formatting | Auto-format SysML/KerML files |
| Semantic tokens | Rich syntax highlighting |
| Inlay hints | Inline type annotations |
| Folding ranges | Collapse code blocks |
| Document links | Clickable imports and type references |
| Diagnostics | Parse errors + semantic errors (undefined refs, duplicates, etc.) |
| Code lens | Inline reference counts |
| Rename | Rename symbols across workspace |
| Workspace symbols | Search symbols across all files |

## Building

```bash
cargo build --release -p syster-lsp
```

## Usage

The LSP server binary can be used with any editor that supports the Language Server Protocol.

For VS Code integration, see the [vscode-lsp extension](https://github.com/jade-codes/syster/tree/main/editors/vscode-lsp).

## License

MIT

## Development

### DevContainer Setup (Recommended)

This project includes a DevContainer configuration for a consistent development environment.

**Using VS Code:**
1. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. Open this repository in VS Code
3. Click "Reopen in Container" when prompted (or use Command Palette: "Dev Containers: Reopen in Container")

**What's included:**
- Rust 1.85+ with 2024 edition
- rust-analyzer, clippy
- GitHub CLI
- All VS Code extensions pre-configured

### Manual Setup

If not using DevContainer:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the LSP server
cargo build --release -p syster-lsp

# Run tests
cargo test --release -p syster-lsp

# Run clippy
cargo clippy -p syster-lsp -- -D warnings
```
