//! Debug test for succession_as_usage (first X then Y;) pattern

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_first_then_hover() {
    // Load stdlib
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Mimic Vehicle Example structure with join nodes
    let source = r#"package Test {
    action def GetInVehicle;
    
    action transportPassenger_1 {
        action driverGetInVehicle : GetInVehicle;
        action passenger1GetInVehicle : GetInVehicle;
        join join1;
        action trigger;
        
        first driverGetInVehicle then join1;
        first passenger1GetInVehicle then join1;
        first join1 then trigger;
    }
}"#;
    // Line 9 (0-indexed): "        first driverGetInVehicle then join1;"
    //                                                           ^-join1 at col 42-47
    
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
    
    // Try hover at various positions on line 9 (the first X then Y line)
    println!("\n=== HOVER TESTS (line 9) ===");
    for col in 8..55 {
        let pos = Position { line: 9, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(80).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}
