//! Debug test for nested public import resolution

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_nested_public_import_hover() {
    // Load stdlib
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Mirror the Vehicle Example structure exactly
    let source = r#"package SimpleVehicleModel {
    public import AttributeDefinitions::*;
    
    package AttributeDefinitions {
        public import ScalarValues::*;
    }
    
    calc def ComputeBSFC {
        return : Real;
    }
}"#;
    
    server.open_document(&uri, source).expect("Should parse");
    
    println!("\n=== ALL SYMBOLS ===");
    for sym in server.workspace().symbol_table().iter_symbols() {
        if sym.qualified_name().starts_with("SimpleVehicleModel") 
           || sym.qualified_name() == "ScalarValues::Real" {
            println!("  {}", sym.qualified_name());
        }
    }
    
    println!("\n=== REFERENCES in test file ===");
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
    
    // Try hover on "return : Real;" (line 8)
    println!("\n=== HOVER TESTS on line 8 (return : Real) ===");
    for col in 8..25 {
        let pos = Position { line: 8, character: col };
        let hover = server.get_hover(&uri, pos);
        if hover.is_some() {
            let result = format!("{:?}", hover.unwrap().contents).chars().take(100).collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}
