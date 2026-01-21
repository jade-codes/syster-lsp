//! Debug test for constraint expression references

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use syster::core::Position as SysterPosition;
use tracing_subscriber;

#[test]
fn test_debug_constraint_expression() {
    // Enable tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    let mut server = LspServer::new();
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    let source = r#"package Test {
    import ISQ::*;
    
    requirement def MassRequirement {
        doc /*The actual mass shall be less than the required mass*/
        attribute massRequired :> ISQ::mass;
        attribute massActual :> ISQ::mass;
        require constraint {massActual <= massRequired}
    }
}"#;

    server.open_document(&uri, source).expect("Should parse");
    
    let st = server.workspace().symbol_table();
    let ref_index = server.workspace().reference_index();
    
    println!("\n=== SYMBOLS IN Test ===");
    for sym in st.iter_symbols() {
        if sym.qualified_name().starts_with("Test::") {
            println!("  {}", sym.qualified_name());
        }
    }
    
    println!("\n=== ALL REFERENCES IN test.sysml ===");
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            if r.file.to_string_lossy().contains("test.sysml") {
                println!("  line={} col={}-{} target='{}' source='{}'", 
                    r.span.start.line, r.span.start.column, r.span.end.column,
                    target, r.source_qname);
            }
        }
    }
    
    println!("\n=== HOVER TESTS ===");
    
    // Line 7: require constraint {massActual <= massRequired}
    // According to trace: massActual at col 28-38, massRequired at col 42-54
    // Check hover on massActual (inside constraint)
    let pos_actual = Position { line: 7, character: 30 };
    let hover_actual = server.get_hover(&uri, pos_actual);
    println!("Hover on 'massActual' (7,30): {:?}", hover_actual.is_some());
    
    // Check hover on massRequired (inside constraint)
    let pos_required = Position { line: 7, character: 45 };
    let hover_required = server.get_hover(&uri, pos_required);
    println!("Hover on 'massRequired' (7,45): {:?}", hover_required.is_some());
    
    println!("\n=== REFERENCE AT POSITION ===");
    let ref_at_30 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(7, 30));
    println!("  Position (7, 30) 'massActual': {:?}", ref_at_30);
    
    let ref_at_45 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(7, 45));
    println!("  Position (7, 45) 'massRequired': {:?}", ref_at_45);
}
