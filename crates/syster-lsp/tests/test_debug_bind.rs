//! Debug test for binding connector pattern

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_bind_hover() {
    // Load stdlib
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Minimal bind example
    let source = r#"package Test {
    part def Engine {
        port fuelCmdPort;
    }
    
    part def Vehicle {
        port fuelCmdPort;
        part engine : Engine;
        bind engine.fuelCmdPort=fuelCmdPort;
    }
}"#;
    // Line 8 (0-indexed): "        bind engine.fuelCmdPort=fuelCmdPort;"
    //                                  ^-engine   ^-fuelCmdPort (first)   ^-fuelCmdPort (second)
    // engine at cols 13-19
    // fuelCmdPort (first) at cols 20-31
    // fuelCmdPort (second) at cols 32-43
    
    server.open_document(&uri, source).expect("Should parse");
    
    println!("\n=== ALL SYMBOLS ===");
    for sym in server.workspace().symbol_table().iter_symbols() {
        if sym.qualified_name().contains("Test") {
            println!("  {} (span: {:?})", sym.qualified_name(), sym.span());
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
    
    // Try hover at various positions on line 8 (the bind line)
    println!("\n=== HOVER TESTS ===");
    for col in 8..50 {
        let pos = Position { line: 8, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(80).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}
