//! Debug test for specializes (:>) pattern

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use syster::core::Position as SysterPosition;
use std::path::PathBuf;

#[test]
fn test_debug_specializes_hover() {
    // Find stdlib path
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()  // crates/
        .parent().unwrap()  // syster-lsp/
        .join("../syster-base/sysml.library");
    
    println!("Stdlib path: {:?}", stdlib_path);
    println!("Stdlib exists: {}", stdlib_path.exists());
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Minimal example of specializes pattern
    let source = r#"package Test {
    import ISQ::*;
    
    part def Vehicle {
        attribute mass :> ISQ::mass;
    }
}"#;
    // Line 4 (0-indexed): "        attribute mass :> ISQ::mass;"
    //                                              ^-- ISQ::mass starts around col 26
    
    server.open_document(&uri, source).expect("Should parse");
    
    println!("\n=== ALL SYMBOLS ===");
    for sym in server.workspace().symbol_table().iter_symbols() {
        println!("  {} (span: {:?})", sym.qualified_name(), sym.span());
    }
    
    println!("\n=== ALL REFERENCES ===");
    let ref_index = server.workspace().reference_index();
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            println!("  line={} col={}-{} target='{}' source='{}'", 
                r.span.start.line, r.span.start.column, r.span.end.column,
                target, r.source_qname);
        }
    }
    
    // Try hover at various positions on line 4
    println!("\n=== HOVER TESTS ===");
    for col in 24..35 {
        let pos = Position { line: 4, character: col };
        let hover = server.get_hover(&uri, pos);
        let result = match &hover {
            Some(h) => format!("{:?}", h.contents).chars().take(80).collect::<String>(),
            None => "None".to_string(),
        };
        println!("  col {}: {}", col, result);
    }
    
    // Check what's at the reference position according to reference index
    println!("\n=== REFERENCE AT POSITION ===");
    let ref_at_26 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(4, 26));
    println!("  Position (4, 26): {:?}", ref_at_26);
    
    let ref_at_30 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(4, 30));
    println!("  Position (4, 30): {:?}", ref_at_30);
}
