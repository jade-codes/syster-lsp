//! Debug test for transitive public import resolution

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_transitive_import_hover() {
    // Load stdlib which has AttributeDefinitions -> ScalarValues chain
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Use transitive import: AttributeDefinitions publicly imports ScalarValues
    let source = r#"package Test {
    private import ISQ::*;
    
    calc def ComputeValue {
        in x : Real;
        return : Real;
    }
}"#;
    // ISQ imports SI which imports ScalarValues, etc.
    
    server.open_document(&uri, source).expect("Should parse");
    
    println!("\n=== CHECKING IMPORTS ===");
    // Check what ISQ exports
    for sym in server.workspace().symbol_table().iter_symbols() {
        if sym.qualified_name() == "ISQ" || sym.qualified_name().starts_with("ISQ::") {
            if !sym.qualified_name().contains("::") {
                println!("  {}", sym.qualified_name());
            }
        }
    }
    
    println!("\n=== HOVER TESTS on line 4 (in x : Real) ===");
    for col in 13..22 {
        let pos = Position { line: 4, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(100).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
    
    println!("\n=== HOVER TESTS on line 5 (return : Real) ===");
    for col in 13..25 {
        let pos = Position { line: 5, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(100).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}
