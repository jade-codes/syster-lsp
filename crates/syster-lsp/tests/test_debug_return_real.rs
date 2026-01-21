//! Debug test for return : Real pattern

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_return_real_hover() {
    // Load stdlib
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Minimal example with return : Real
    let source = r#"package Test {
    private import ScalarValues::*;
    
    calc def ComputeValue {
        in x : Real;
        return : Real;
    }
}"#;
    // Line 5 (0-indexed): "        return : Real;"
    //                              ^return   ^Real at col 17-21
    
    server.open_document(&uri, source).expect("Should parse");
    
    println!("\n=== ALL SYMBOLS (Test package only) ===");
    for sym in server.workspace().symbol_table().iter_symbols() {
        if sym.qualified_name().contains("Test") {
            println!("  {}", sym.qualified_name());
        }
    }
    
    println!("\n=== ALL REFERENCES ===");
    let ref_index = server.workspace().reference_index();
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            if r.file == PathBuf::from("/test.sysml") {
                println!("  line={} col={}-{} target='{}' source='{}'", 
                    r.span.start.line, r.span.start.column, r.span.end.column,
                    target, r.source_qname);
            }
        }
    }
    
    // Try hover on line 5 (return : Real;)
    println!("\n=== HOVER TESTS on line 5 ===");
    for col in 8..30 {
        let pos = Position { line: 5, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(100).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
    
    // Also check line 4 (in x : Real;)
    println!("\n=== HOVER TESTS on line 4 (in x : Real) ===");
    for col in 8..25 {
        let pos = Position { line: 4, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(100).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}
