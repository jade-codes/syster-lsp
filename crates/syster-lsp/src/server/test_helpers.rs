//! Test helpers for LspServer tests
//!
//! These helpers abstract away internal implementation details so tests
//! don't break when we refactor the underlying architecture.
//!
//! This module is public so integration tests can use these helpers.

#![allow(dead_code)]

use crate::server::LspServer;
use std::path::PathBuf;
use syster::hir::{HirSymbol, ResolveResult, Resolver, SymbolKind, TypeRef};

/// Create an LspServer without stdlib (fast, for most unit tests)
pub fn create_server() -> LspServer {
    LspServer::with_config(false, None)
}

/// Extension trait for LspServer test helpers
///
/// These methods provide test-friendly access to internal state without
/// exposing implementation details like Workspace or SymbolTable directly.
pub trait LspServerTestExt {
    /// Get the number of symbols in the index
    fn symbol_count(&mut self) -> usize;

    /// Check if a symbol with the given name exists
    fn has_symbol(&mut self, name: &str) -> bool;

    /// Check if a symbol with the given qualified name exists
    fn has_qualified_symbol(&mut self, qname: &str) -> bool;

    /// Get all symbol names (for debugging)
    fn symbol_names(&mut self) -> Vec<String>;

    /// Get all qualified symbol names
    fn qualified_symbol_names(&mut self) -> Vec<String>;

    /// Find symbols matching a predicate
    fn find_symbols<F>(&mut self, predicate: F) -> Vec<SymbolSnapshot>
    where
        F: Fn(&HirSymbol) -> bool;

    /// Find a symbol by name
    fn find_symbol(&mut self, name: &str) -> Option<SymbolSnapshot>;

    /// Find a symbol by qualified name  
    fn find_symbol_qualified(&mut self, qname: &str) -> Option<SymbolSnapshot>;

    /// Iterate over all symbols (returns snapshots to avoid lifetime issues)
    fn all_symbols(&mut self) -> Vec<SymbolSnapshot>;

    /// Print all symbols (for debugging)
    fn print_all_symbols(&mut self);

    /// Print symbols matching a filter
    fn print_symbols_filtered(&mut self, filter: &str);

    /// Get the number of files loaded
    fn loaded_file_count(&self) -> usize;

    /// Check if a file path is loaded
    fn has_file(&self, path: &str) -> bool;

    /// Check if a file path (PathBuf) is loaded
    fn has_file_path(&self, path: &std::path::Path) -> bool;

    /// Get all loaded file paths
    fn loaded_file_paths(&self) -> Vec<PathBuf>;

    /// Get count of type references to a target
    fn reference_count(&mut self, target: &str) -> usize;

    /// Get all type references
    fn all_references(&mut self) -> Vec<TypeRefSnapshot>;

    /// Get references in a specific file
    fn references_in_file(&mut self, file_path: &str) -> Vec<TypeRefSnapshot>;

    /// Get reference at a specific position (line, col are 0-indexed)
    fn reference_at_position(
        &mut self,
        file_path: &str,
        line: u32,
        col: u32,
    ) -> Option<TypeRefSnapshot>;

    /// Get all type reference targets
    fn all_reference_targets(&mut self) -> Vec<String>;

    /// Print all references (for debugging)
    fn print_all_references(&mut self);

    /// Print references in a file
    fn print_references_in_file(&mut self, file_path: &str);

    /// Check if stdlib is loaded (has many files)
    fn has_stdlib_loaded(&self) -> bool;

    /// Resolve a name from a given scope using visibility maps
    /// This handles imports properly, unlike find_symbol_qualified which does direct lookup
    fn resolve_name(&mut self, scope: &str, name: &str) -> Option<SymbolSnapshot>;
}

/// A snapshot of symbol data for tests (owned, no lifetime issues)
#[derive(Clone, Debug)]
pub struct SymbolSnapshot {
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub supertypes: Vec<String>,
    pub doc: Option<String>,
    pub type_refs: Vec<TypeRefSnapshot>,
}

impl From<&HirSymbol> for SymbolSnapshot {
    fn from(sym: &HirSymbol) -> Self {
        Self {
            name: sym.name.to_string(),
            qualified_name: sym.qualified_name.to_string(),
            kind: sym.kind,
            start_line: sym.start_line,
            start_col: sym.start_col,
            end_line: sym.end_line,
            end_col: sym.end_col,
            supertypes: sym.supertypes.iter().map(|s| s.to_string()).collect(),
            doc: sym.doc.as_ref().map(|d| d.to_string()),
            type_refs: sym
                .type_refs
                .iter()
                .flat_map(|trk| trk.as_refs())
                .map(TypeRefSnapshot::from)
                .collect(),
        }
    }
}

/// A snapshot of type reference data for tests
#[derive(Clone, Debug)]
pub struct TypeRefSnapshot {
    pub target: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// The symbol that contains this reference (for context)
    pub source_symbol: Option<String>,
    /// The file containing this reference
    pub file_path: Option<String>,
}

impl From<&TypeRef> for TypeRefSnapshot {
    fn from(tr: &TypeRef) -> Self {
        Self {
            target: tr.target.to_string(),
            start_line: tr.start_line,
            start_col: tr.start_col,
            end_line: tr.end_line,
            end_col: tr.end_col,
            source_symbol: None,
            file_path: None,
        }
    }
}

impl LspServerTestExt for LspServer {
    fn symbol_count(&mut self) -> usize {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().count();
        result
    }

    fn has_symbol(&mut self, name: &str) -> bool {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().any(|s| s.name.as_ref() == name);
        result
    }

    fn has_qualified_symbol(&mut self, qname: &str) -> bool {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().any(|s| s.qualified_name.as_ref() == qname);
        result
    }

    fn symbol_names(&mut self) -> Vec<String> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().map(|s| s.name.to_string()).collect();
        result
    }

    fn qualified_symbol_names(&mut self) -> Vec<String> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().map(|s| s.qualified_name.to_string()).collect();
        result
    }

    fn find_symbols<F>(&mut self, predicate: F) -> Vec<SymbolSnapshot>
    where
        F: Fn(&HirSymbol) -> bool,
    {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().filter(|s| predicate(s)).map(SymbolSnapshot::from).collect();
        result
    }

    fn find_symbol(&mut self, name: &str) -> Option<SymbolSnapshot> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().find(|s| s.name.as_ref() == name).map(SymbolSnapshot::from);
        result
    }

    fn find_symbol_qualified(&mut self, qname: &str) -> Option<SymbolSnapshot> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().find(|s| s.qualified_name.as_ref() == qname).map(SymbolSnapshot::from);
        result
    }

    fn all_symbols(&mut self) -> Vec<SymbolSnapshot> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let result = index.all_symbols().map(SymbolSnapshot::from).collect();
        result
    }

    fn print_all_symbols(&mut self) {
        println!("\n=== ALL SYMBOLS ===");
        for sym in self.all_symbols() {
            println!(
                "  {} (line {}-{})",
                sym.qualified_name, sym.start_line, sym.end_line
            );
        }
    }

    fn print_symbols_filtered(&mut self, filter: &str) {
        println!("\n=== SYMBOLS (filtered: '{}') ===", filter);
        for sym in self.all_symbols() {
            if sym.qualified_name.contains(filter) {
                println!(
                    "  {} (line {}-{})",
                    sym.qualified_name, sym.start_line, sym.end_line
                );
            }
        }
    }

    fn loaded_file_count(&self) -> usize {
        self.analysis_host.file_count()
    }

    fn has_file(&self, path: &str) -> bool {
        self.analysis_host.has_file(path)
    }

    fn has_file_path(&self, path: &std::path::Path) -> bool {
        self.analysis_host.has_file_path(path)
    }

    fn loaded_file_paths(&self) -> Vec<PathBuf> {
        self.analysis_host.files().keys().cloned().collect()
    }

    fn reference_count(&mut self, target: &str) -> usize {
        let analysis = self.analysis_host.analysis();
        analysis
            .symbol_index()
            .all_symbols()
            .flat_map(|s| s.type_refs.iter())
            .flat_map(|trk| trk.as_refs())
            .filter(|tr| tr.target.as_ref() == target)
            .count()
    }

    fn all_references(&mut self) -> Vec<TypeRefSnapshot> {
        let analysis = self.analysis_host.analysis();
        let mut refs = Vec::new();
        for sym in analysis.symbol_index().all_symbols() {
            let file_path = analysis.get_file_path(sym.file).map(|s| s.to_string());
            for trk in &sym.type_refs {
                for tr in trk.as_refs() {
                    let mut snapshot = TypeRefSnapshot::from(tr);
                    snapshot.source_symbol = Some(sym.qualified_name.to_string());
                    snapshot.file_path = file_path.clone();
                    refs.push(snapshot);
                }
            }
        }
        refs
    }

    fn references_in_file(&mut self, file_path: &str) -> Vec<TypeRefSnapshot> {
        self.all_references()
            .into_iter()
            .filter(|r| r.file_path.as_deref() == Some(file_path))
            .collect()
    }

    fn reference_at_position(
        &mut self,
        file_path: &str,
        line: u32,
        col: u32,
    ) -> Option<TypeRefSnapshot> {
        self.references_in_file(file_path).into_iter().find(|r| {
            let after_start = line > r.start_line || (line == r.start_line && col >= r.start_col);
            let before_end = line < r.end_line || (line == r.end_line && col <= r.end_col);
            after_start && before_end
        })
    }

    fn all_reference_targets(&mut self) -> Vec<String> {
        let analysis = self.analysis_host.analysis();
        let mut targets: Vec<_> = analysis
            .symbol_index()
            .all_symbols()
            .flat_map(|s| s.type_refs.iter())
            .flat_map(|trk| trk.as_refs())
            .map(|tr| tr.target.to_string())
            .collect();
        targets.sort();
        targets.dedup();
        targets
    }

    fn print_all_references(&mut self) {
        println!("\n=== ALL REFERENCES ===");
        for r in self.all_references() {
            println!(
                "  line={} col={}-{} target='{}' source='{}'",
                r.start_line,
                r.start_col,
                r.end_col,
                r.target,
                r.source_symbol.as_deref().unwrap_or("?")
            );
        }
    }

    fn print_references_in_file(&mut self, file_path: &str) {
        println!("\n=== REFERENCES IN {} ===", file_path);
        for r in self.references_in_file(file_path) {
            println!(
                "  line={} col={}-{} target='{}' source='{}'",
                r.start_line,
                r.start_col,
                r.end_col,
                r.target,
                r.source_symbol.as_deref().unwrap_or("?")
            );
        }
    }

    fn has_stdlib_loaded(&self) -> bool {
        // Stdlib has 50+ files, so if we have many files, stdlib is likely loaded
        self.analysis_host.file_count() > 50
    }

    fn resolve_name(&mut self, scope: &str, name: &str) -> Option<SymbolSnapshot> {
        let analysis = self.analysis_host.analysis();
        let index = analysis.symbol_index();
        let resolver = Resolver::new(index).with_scope(scope);
        match resolver.resolve(name) {
            ResolveResult::Found(sym) => Some(SymbolSnapshot::from(&sym)),
            _ => None,
        }
    }
}
