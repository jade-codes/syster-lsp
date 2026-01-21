//! Debug test for specializes with expression

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use syster::core::Position as SysterPosition;
use tracing_subscriber;

#[test]
fn test_debug_specializes_expression_no_stdlib() {
    // Enable tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    let mut server = LspServer::new(); // NO stdlib to make it fast
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Define our own minimal definitions
    let source = r#"package ISQ {
    attribute def ScalarQuantityValue;
    attribute scalarQuantities : ScalarQuantityValue;
    alias time for duration;
    attribute duration : ScalarQuantityValue;
    attribute distance : ScalarQuantityValue;
}

package Test {
    import ISQ::*;
    
    // Line 10: timePerDistance :> scalarQuantities = time / distance;
    attribute timePerDistance :> scalarQuantities = time / distance;
}"#;

    server.open_document(&uri, source).expect("Should parse");
    
    let st = server.workspace().symbol_table();
    let ref_index = server.workspace().reference_index();
    
    // Check symbols
    println!("\n=== ALL SYMBOLS ===");
    for sym in st.iter_symbols() {
        println!("  {} (scope_id={})", sym.qualified_name(), sym.scope_id());
    }
    
    println!("\n=== ALL REFERENCES ===");
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            println!("  line={} col={}-{} target='{}' source='{}'", 
                r.span.start.line, r.span.start.column, r.span.end.column,
                target, r.source_qname);
        }
    }
    
    println!("\n=== HOVER TESTS ===");
    
    // Line 12: attribute timePerDistance :> scalarQuantities = time / distance;
    // Positions:  col 4-19=timePerDistance, 24-39=scalarQuantities, 42-46=time, 49-57=distance
    
    // Test hover on scalarQuantities
    let pos_scalar = Position { line: 12, character: 35 };
    let hover_scalar = server.get_hover(&uri, pos_scalar);
    println!("Hover on 'scalarQuantities' (12,35): {:?}", hover_scalar.is_some());
    
    // Test hover on time
    let pos_time = Position { line: 12, character: 52 };
    let hover_time = server.get_hover(&uri, pos_time);
    println!("Hover on 'time' (12,52): {:?}", hover_time.is_some());
    
    // Test hover on distance
    let pos_dist = Position { line: 12, character: 60 };
    let hover_dist = server.get_hover(&uri, pos_dist);
    println!("Hover on 'distance' (12,60): {:?}", hover_dist.is_some());
    
    // Check reference at position for line 12
    println!("\n=== REFERENCE AT POSITION ===");
    let ref_at_35 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(12, 35));
    println!("  Position (12, 35): {:?}", ref_at_35);
    
    let ref_at_52 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(12, 52));
    println!("  Position (12, 52): {:?}", ref_at_52);
    
    let ref_at_60 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(12, 60));
    println!("  Position (12, 60): {:?}", ref_at_60);
}
