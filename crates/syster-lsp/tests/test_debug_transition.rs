//! Debug test for transition references

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use syster::core::Position as SysterPosition;
use tracing_subscriber;

#[test]
fn test_debug_transition_references() {
    // Enable tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
    
    let mut server = LspServer::new();
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    let source = r#"package Test {
    enum def IgnitionOnOff {
        on; off;
    }
    
    item def IgnitionCmd {
        attribute ignitionOnOff : IgnitionOnOff;
    }
    
    item def StartSignal;
    
    port def IgnitionCmdPort {
        in item ignitionCmd : IgnitionCmd;
    }
    
    state def EngineStates {
        entry; then off;
        
        state off;
        state starting;
        state running;
        
        attribute brakePedalDepressed : ScalarValues::Boolean;
        port ignitionCmdPort : IgnitionCmdPort;
        ref controller;
        
        transition off_To_starting
            first off
            accept ignitionCmd:IgnitionCmd via ignitionCmdPort
                if ignitionCmd.ignitionOnOff==IgnitionOnOff::on and brakePedalDepressed
            do send new StartSignal() to controller
            then starting;
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
    
    // Line 27: first off
    let pos_off = Position { line: 27, character: 18 };
    let hover_off = server.get_hover(&uri, pos_off);
    println!("Hover on 'off' (27,18) first target: {:?}", hover_off.is_some());
    
    // Line 28: accept ignitionCmd:IgnitionCmd via ignitionCmdPort
    let pos_port = Position { line: 28, character: 55 };
    let hover_port = server.get_hover(&uri, pos_port);
    println!("Hover on 'ignitionCmdPort' (28,55) via port: {:?}", hover_port.is_some());
    
    // Line 29: if ignitionCmd.ignitionOnOff==IgnitionOnOff::on
    let pos_enum = Position { line: 29, character: 55 };
    let hover_enum = server.get_hover(&uri, pos_enum);
    println!("Hover on 'IgnitionOnOff::on' (29,55): {:?}", hover_enum.is_some());
    
    // Line 29: brakePedalDepressed
    let pos_brake = Position { line: 29, character: 75 };
    let hover_brake = server.get_hover(&uri, pos_brake);
    println!("Hover on 'brakePedalDepressed' (29,75): {:?}", hover_brake.is_some());
    
    // Line 30: do send new StartSignal() to controller
    let pos_signal = Position { line: 30, character: 27 };
    let hover_signal = server.get_hover(&uri, pos_signal);
    println!("Hover on 'StartSignal' (30,27): {:?}", hover_signal.is_some());
    
    let pos_controller = Position { line: 30, character: 47 };
    let hover_controller = server.get_hover(&uri, pos_controller);
    println!("Hover on 'controller' (30,47): {:?}", hover_controller.is_some());
    
    // Line 31: then starting
    let pos_starting = Position { line: 31, character: 17 };
    let hover_starting = server.get_hover(&uri, pos_starting);
    println!("Hover on 'starting' (31,17) then target: {:?}", hover_starting.is_some());
    
    println!("\n=== REFERENCE AT POSITION ===");
    let ref_at_off = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(27, 18));
    println!("  Position (27, 18) 'off': {:?}", ref_at_off);
    
    let ref_at_port = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(28, 55));
    println!("  Position (28, 55) 'ignitionCmdPort': {:?}", ref_at_port);
    
    let ref_at_starting = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(31, 17));
    println!("  Position (31, 17) 'starting': {:?}", ref_at_starting);
}
