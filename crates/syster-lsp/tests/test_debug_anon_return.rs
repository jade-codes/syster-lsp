//! Debug test for anonymous return : Real pattern (NO name)

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_debug_anon_return_hover() {
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // EXACT syntax from Vehicle Example line 1088-1091
    let source = r#"package Test {
    public import ScalarValues::*;
    
    calc def ComputeBSFC {
        in engine: Engine;
        return : Real;
    }
    
    part def Engine;
}"#;
    // Line 5: "        return : Real;"
    //                  ^return   ^: at col 15   ^Real at col 17-21
    
    server.open_document(&uri, source).expect("Should parse");
    
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
    
    println!("\n=== HOVER TESTS on line 5 (return : Real;) ===");
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
}
