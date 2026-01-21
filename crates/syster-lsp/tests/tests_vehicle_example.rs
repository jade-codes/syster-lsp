//! Comprehensive test that scans SimpleVehicleModel.sysml and reports ALL failing hovers.
//!
//! Run with: cargo test --test test_vehicle_example_coverage -- --nocapture

use async_lsp::lsp_types::{Position, Url};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syster_lsp::server::LspServer;

/// A failing reference with context
#[derive(Debug)]
struct FailingReference {
    line: usize,
    col_start: usize,
    col_end: usize,
    target: String,
    source: String,
    line_text: String,
}

#[test]
fn test_vehicle_example_all_references_have_hover() {
    // Load the vehicle example file
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // syster-lsp/
        .join("../syster-base/tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    
    let source = fs::read_to_string(&file_path)
        .expect("Should be able to read SimpleVehicleModel.sysml");
    
    let lines: Vec<&str> = source.lines().collect();
    
    // Load stdlib for proper resolution
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // syster-lsp/
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    server.open_document(&uri, &source).expect("Should parse document");
    
    // Get all references from the reference index
    let ref_index = server.workspace().reference_index();
    
    println!("\n{}", "=".repeat(80));
    println!("VEHICLE EXAMPLE REFERENCE COVERAGE TEST");
    println!("{}\n", "=".repeat(80));
    
    // Collect all references with their targets
    let mut all_refs: Vec<(String, String, usize, usize, usize)> = Vec::new(); // (target, source, line, col_start, col_end)
    
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            if r.file == PathBuf::from("/test.sysml") {
                all_refs.push((
                    target.to_string(),
                    r.source_qname.clone(),
                    r.span.start.line,
                    r.span.start.column,
                    r.span.end.column,
                ));
            }
        }
    }
    
    // Sort by line number for easier reading
    all_refs.sort_by_key(|(_, _, line, col, _)| (*line, *col));
    
    println!("Total references found: {}\n", all_refs.len());
    
    // Track failures by category
    let mut passing_count = 0;
    let mut failing_by_pattern: HashMap<String, Vec<FailingReference>> = HashMap::new();
    
    // Test each reference
    for (target, source, line, col_start, col_end) in &all_refs {
        let pos = Position { 
            line: *line as u32, 
            character: *col_start as u32,
        };
        
        let hover_result = server.get_hover(&uri, pos);
        let line_text = lines.get(*line).unwrap_or(&"").to_string();
        
        match hover_result {
            Some(hover) => {
                // Check if hover resolves to an actual symbol (not just the source element)
                let hover_content = format!("{:?}", hover.contents);
                
                // Check if it resolves to the target or just falls back to containing element
                let is_simple_name = !target.contains("::");
                let resolved_to_target = hover_content.contains(target) || 
                    (is_simple_name && hover_content.contains(&format!("::{}", target)));
                
                if is_simple_name && !resolved_to_target {
                    // This is a simple name that didn't resolve - likely a problem
                    let failure = FailingReference {
                        line: *line,
                        col_start: *col_start,
                        col_end: *col_end,
                        target: target.clone(),
                        source: source.clone(),
                        line_text: line_text.trim().to_string(),
                    };
                    
                    // Categorize by pattern
                    let pattern = categorize_pattern(&line_text, &target);
                    failing_by_pattern.entry(pattern).or_default().push(failure);
                } else {
                    passing_count += 1;
                }
            }
            None => {
                let failure = FailingReference {
                    line: *line,
                    col_start: *col_start,
                    col_end: *col_end,
                    target: target.clone(),
                    source: source.clone(),
                    line_text: line_text.trim().to_string(),
                };
                
                let pattern = categorize_pattern(&line_text, &target);
                failing_by_pattern.entry(pattern).or_default().push(failure);
            }
        }
    }
    
    // Print results grouped by pattern
    println!("\n{}", "=".repeat(80));
    println!("RESULTS SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Passing: {}", passing_count);
    println!("Failing patterns: {}", failing_by_pattern.len());
    
    let total_failures: usize = failing_by_pattern.values().map(|v| v.len()).sum();
    let num_patterns = failing_by_pattern.len();
    println!("Total failing: {}", total_failures);
    
    if !failing_by_pattern.is_empty() {
        println!("\n{}", "=".repeat(80));
        println!("FAILING PATTERNS (grouped)");
        println!("{}\n", "=".repeat(80));
        
        let mut patterns: Vec<_> = failing_by_pattern.into_iter().collect();
        patterns.sort_by_key(|(_, refs)| std::cmp::Reverse(refs.len()));
        
        for (pattern, refs) in patterns {
            println!("\n## Pattern: {} ({} occurrences)", pattern, refs.len());
            println!("{}", "-".repeat(60));
            
            // Show first 5 examples
            for (i, r) in refs.iter().take(5).enumerate() {
                println!("  {}. Line {}: {}", i + 1, r.line + 1, r.line_text);
                println!("     Target: '{}' at col {}-{}", r.target, r.col_start, r.col_end);
                println!("     Source: {}", r.source);
            }
            
            if refs.len() > 5 {
                println!("  ... and {} more", refs.len() - 5);
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("END OF REPORT");
    println!("{}\n", "=".repeat(80));
    
    // Fail if there are any unresolved references
    assert!(
        total_failures == 0, 
        "Found {} failing references across {} patterns. See report above for details.", 
        total_failures, 
        num_patterns
    );
}

/// Categorize a reference into a pattern for grouping
fn categorize_pattern(line_text: &str, _target: &str) -> String {
    let trimmed = line_text.trim();
    
    if trimmed.contains("redefines") {
        return "redefines".to_string();
    }
    if trimmed.contains("subsets") {
        return "subsets".to_string();
    }
    if trimmed.contains(":>") && !trimmed.contains("::>") {
        return "specializes (:>)".to_string();
    }
    if trimmed.contains("::>") {
        return "featured by (::>)".to_string();
    }
    if trimmed.contains(":>>") {
        return "redefines short (:>>)".to_string();
    }
    if trimmed.contains(" :~") || trimmed.contains(":~") {
        return "conjugate port (~)".to_string();
    }
    if trimmed.contains("accept ") {
        return "accept (state machine)".to_string();
    }
    if trimmed.contains("send ") && trimmed.contains(" to ") {
        return "send ... to".to_string();
    }
    if trimmed.contains(" via ") {
        return "via (port)".to_string();
    }
    if trimmed.contains("first ") {
        return "first (transition)".to_string();
    }
    if trimmed.contains("then ") {
        return "then (transition)".to_string();
    }
    if trimmed.contains("ref ") {
        return "ref usage".to_string();
    }
    if trimmed.contains(" = ") || trimmed.contains("==") {
        return "expression/value".to_string();
    }
    if trimmed.contains("constraint") {
        return "constraint".to_string();
    }
    if trimmed.contains(" : ") {
        return "typing (:)".to_string();
    }
    if trimmed.starts_with("import ") || trimmed.contains("import ") {
        return "import".to_string();
    }
    
    "other".to_string()
}