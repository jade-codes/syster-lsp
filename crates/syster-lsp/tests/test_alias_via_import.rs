//! Test alias resolution when target is available via import

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use syster::core::Position as SysterPosition;
use tracing_subscriber;

#[test]
fn test_alias_target_via_import() {
    // Enable tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    let mut server = LspServer::with_config(false, None); // NO stdlib to make it fast
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Simulate the real structure: ISQBase defines duration, ISQSpaceTime imports it and creates alias
    let source = r#"package ISQBase {
    attribute def ScalarQuantityValue;
    attribute scalarQuantities : ScalarQuantityValue;
    attribute duration : ScalarQuantityValue;
    attribute distance : ScalarQuantityValue;
}

package ISQSpaceTime {
    public import ISQBase::*;
    
    // This mirrors the real ISQSpaceTime: alias time for duration (which is imported)
    alias time for duration;
}

package Test {
    import ISQSpaceTime::*;
    
    // Line 15: timePerDistance :> scalarQuantities = time / distance;
    attribute timePerDistance :> scalarQuantities = time / distance;
}"#;

    server.open_document(&uri, source).expect("Should parse");
    
    let st = server.workspace().symbol_table();
    let ref_index = server.workspace().reference_index();
    
    // Check that ISQSpaceTime::time alias exists
    println!("\n=== KEY SYMBOLS ===");
    if let Some(time_sym) = st.find_by_qualified_name("ISQSpaceTime::time") {
        println!("ISQSpaceTime::time: {} (scope_id={})", time_sym.qualified_name(), time_sym.scope_id());
    } else {
        println!("ISQSpaceTime::time: NOT FOUND");
    }
    
    if let Some(dur_sym) = st.find_by_qualified_name("ISQBase::duration") {
        println!("ISQBase::duration: {} (scope_id={})", dur_sym.qualified_name(), dur_sym.scope_id());
    } else {
        println!("ISQBase::duration: NOT FOUND");
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
    
    // Line 19 (0-indexed: 18): attribute timePerDistance :> scalarQuantities = time / distance;
    // Column analysis:
    // 0123456789...
    // "    attribute timePerDistance :> scalarQuantities = time / distance;"
    //                                  ^28             ^45   ^52  ^57
    
    // Test hover on scalarQuantities (starts around col 32)
    let pos_scalar = Position { line: 18, character: 35 };
    let hover_scalar = server.get_hover(&uri, pos_scalar);
    println!("Hover on 'scalarQuantities' (18,35): {:?}", hover_scalar.is_some());
    
    // Test hover on time (this is the alias that should resolve to ISQBase::duration)
    let pos_time = Position { line: 18, character: 52 };
    let hover_time = server.get_hover(&uri, pos_time);
    println!("Hover on 'time' (18,52): {:?}", hover_time.is_some());
    
    // Test hover on distance
    let pos_dist = Position { line: 18, character: 60 };
    let hover_dist = server.get_hover(&uri, pos_dist);
    println!("Hover on 'distance' (18,60): {:?}", hover_dist.is_some());
    
    // Check reference at position for line 18
    println!("\n=== REFERENCE AT POSITION ===");
    let ref_at_35 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(18, 35));
    println!("  Position (18, 35): {:?}", ref_at_35);
    
    let ref_at_52 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(18, 52));
    println!("  Position (18, 52): {:?}", ref_at_52);
    
    let ref_at_60 = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(18, 60));
    println!("  Position (18, 60): {:?}", ref_at_60);
    
    // Assertions
    assert!(hover_scalar.is_some(), "hover on scalarQuantities should work");
    assert!(hover_time.is_some(), "hover on time (alias via import) should work");
    assert!(hover_dist.is_some(), "hover on distance should work");
}
