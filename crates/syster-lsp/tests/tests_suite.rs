//! Consolidated integration tests for syster-lsp
//!
//! This file consolidates all the individual debug/test files into organized modules.
//! Run with: cargo test --test tests_suite -- --nocapture

use async_lsp::lsp_types::{Position, Url};
use std::path::PathBuf;
use syster::core::Position as SysterPosition;
use syster_lsp::server::LspServer;

// ============================================================
// COMMON HELPER FUNCTIONS
// ============================================================

/// Get the path to the stdlib directory
fn get_stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("../syster-base/sysml.library")
}

/// Create a server without stdlib (fast tests)
fn create_server_no_stdlib() -> LspServer {
    LspServer::with_config(false, None)
}

/// Create a server with stdlib loaded
fn create_server_with_stdlib() -> LspServer {
    LspServer::with_config(true, Some(get_stdlib_path()))
}

/// Open a document in the server and return the URI
fn open_test_document(server: &mut LspServer, source: &str) -> Url {
    let uri = Url::parse("file:///test.sysml").unwrap();
    server.open_document(&uri, source).expect("Should parse");
    uri
}

/// Print all symbols from the workspace that match a filter
fn print_symbols_filtered(server: &LspServer, filter: &str) {
    println!("\n=== SYMBOLS (filtered: '{}') ===", filter);
    for sym in server.workspace().symbol_table().iter_symbols() {
        if sym.qualified_name().contains(filter) {
            println!("  {} (span: {:?})", sym.qualified_name(), sym.span());
        }
    }
}

/// Print all symbols from the workspace
fn print_all_symbols(server: &LspServer) {
    println!("\n=== ALL SYMBOLS ===");
    for sym in server.workspace().symbol_table().iter_symbols() {
        println!("  {} (span: {:?})", sym.qualified_name(), sym.span());
    }
}

/// Print all references in a test file
fn print_references_in_test_file(server: &LspServer) {
    println!("\n=== ALL REFERENCES ===");
    let ref_index = server.workspace().reference_index();
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            if r.file == std::path::Path::new("/test.sysml") {
                println!(
                    "  line={} col={}-{} target='{}' source='{}'",
                    r.span.start.line,
                    r.span.start.column,
                    r.span.end.column,
                    target,
                    r.source_qname
                );
            }
        }
    }
}

/// Test hover at a range of columns on a given line
fn test_hover_at_columns(server: &LspServer, uri: &Url, line: u32, col_start: u32, col_end: u32) {
    println!("\n=== HOVER TESTS (line {}) ===", line);
    for col in col_start..col_end {
        let pos = Position {
            line,
            character: col,
        };
        let hover = server.get_hover(uri, pos);
        if let Some(h) = hover {
            let result = format!("{:?}", h.contents)
                .chars()
                .take(80)
                .collect::<String>();
            println!("  col {}: {}", col, result);
        } else {
            println!("  col {}: None", col);
        }
    }
}

/// Check reference at a specific position
fn check_reference_at_position(server: &LspServer, line: usize, col: usize, desc: &str) {
    let ref_index = server.workspace().reference_index();
    let ref_at_pos = ref_index.get_reference_at_position("/test.sysml", SysterPosition::new(line, col));
    println!("  Position ({}, {}) '{}': {:?}", line, col, desc, ref_at_pos);
}

// ============================================================
// DEBUG TESTS - NO STDLIB (Fast)
// ============================================================

mod debug_no_stdlib {
    use super::*;

    #[test]
    fn test_alias_target_via_import() {
        let mut server = create_server_no_stdlib();
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

        let uri = open_test_document(&mut server, source);
        let _ref_index = server.workspace().reference_index();

        println!("\n=== KEY SYMBOLS ===");
        let st = server.workspace().symbol_table();
        if let Some(time_sym) = st.find_by_qualified_name("ISQSpaceTime::time") {
            println!(
                "ISQSpaceTime::time: {} (scope_id={})",
                time_sym.qualified_name(),
                time_sym.scope_id()
            );
        } else {
            println!("ISQSpaceTime::time: NOT FOUND");
        }

        print_references_in_test_file(&server);

        println!("\n=== HOVER TESTS ===");
        // Test hover on scalarQuantities (starts around col 32)
        let pos_scalar = Position {
            line: 18,
            character: 35,
        };
        let hover_scalar = server.get_hover(&uri, pos_scalar);
        println!(
            "Hover on 'scalarQuantities' (18,35): {:?}",
            hover_scalar.is_some()
        );

        // Test hover on time
        let pos_time = Position {
            line: 18,
            character: 52,
        };
        let hover_time = server.get_hover(&uri, pos_time);
        println!("Hover on 'time' (18,52): {:?}", hover_time.is_some());

        // Test hover on distance
        let pos_dist = Position {
            line: 18,
            character: 60,
        };
        let hover_dist = server.get_hover(&uri, pos_dist);
        println!("Hover on 'distance' (18,60): {:?}", hover_dist.is_some());

        // Assertions
        assert!(hover_scalar.is_some(), "hover on scalarQuantities should work");
        assert!(
            hover_time.is_some(),
            "hover on time (alias via import) should work"
        );
        assert!(hover_dist.is_some(), "hover on distance should work");
    }

    #[test]
    fn test_constraint_expression() {
        let mut server = create_server_no_stdlib();
        let source = r#"package Test {
    import ISQ::*;
    
    requirement def MassRequirement {
        doc /*The actual mass shall be less than the required mass*/
        attribute massRequired :> ISQ::mass;
        attribute massActual :> ISQ::mass;
        require constraint {massActual <= massRequired}
    }
}"#;

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test::");
        print_references_in_test_file(&server);

        println!("\n=== HOVER TESTS ===");
        // Line 7: require constraint {massActual <= massRequired}
        let pos_actual = Position {
            line: 7,
            character: 30,
        };
        let hover_actual = server.get_hover(&uri, pos_actual);
        println!(
            "Hover on 'massActual' (7,30): {:?}",
            hover_actual.is_some()
        );

        let pos_required = Position {
            line: 7,
            character: 45,
        };
        let hover_required = server.get_hover(&uri, pos_required);
        println!(
            "Hover on 'massRequired' (7,45): {:?}",
            hover_required.is_some()
        );

        println!("\n=== REFERENCE AT POSITION ===");
        check_reference_at_position(&server, 7, 30, "massActual");
        check_reference_at_position(&server, 7, 45, "massRequired");
    }

    #[test]
    fn test_specializes_expression_no_stdlib() {
        let mut server = create_server_no_stdlib();
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

        let uri = open_test_document(&mut server, source);

        print_all_symbols(&server);
        print_references_in_test_file(&server);

        println!("\n=== HOVER TESTS ===");
        // Test hover on scalarQuantities
        let pos_scalar = Position {
            line: 12,
            character: 35,
        };
        let hover_scalar = server.get_hover(&uri, pos_scalar);
        println!(
            "Hover on 'scalarQuantities' (12,35): {:?}",
            hover_scalar.is_some()
        );

        // Test hover on time
        let pos_time = Position {
            line: 12,
            character: 52,
        };
        let hover_time = server.get_hover(&uri, pos_time);
        println!("Hover on 'time' (12,52): {:?}", hover_time.is_some());

        // Test hover on distance
        let pos_dist = Position {
            line: 12,
            character: 60,
        };
        let hover_dist = server.get_hover(&uri, pos_dist);
        println!("Hover on 'distance' (12,60): {:?}", hover_dist.is_some());
    }

    #[test]
    fn test_transition_references() {
        let mut server = create_server_no_stdlib();
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

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test::");
        print_references_in_test_file(&server);

        println!("\n=== HOVER TESTS ===");
        // Line 27: first off
        let pos_off = Position {
            line: 27,
            character: 18,
        };
        let hover_off = server.get_hover(&uri, pos_off);
        println!(
            "Hover on 'off' (27,18) first target: {:?}",
            hover_off.is_some()
        );

        // Line 28: accept ignitionCmd:IgnitionCmd via ignitionCmdPort
        let pos_port = Position {
            line: 28,
            character: 55,
        };
        let hover_port = server.get_hover(&uri, pos_port);
        println!(
            "Hover on 'ignitionCmdPort' (28,55) via port: {:?}",
            hover_port.is_some()
        );

        // Line 31: then starting
        let pos_starting = Position {
            line: 31,
            character: 17,
        };
        let hover_starting = server.get_hover(&uri, pos_starting);
        println!(
            "Hover on 'starting' (31,17) then target: {:?}",
            hover_starting.is_some()
        );

        println!("\n=== REFERENCE AT POSITION ===");
        check_reference_at_position(&server, 27, 18, "off");
        check_reference_at_position(&server, 28, 55, "ignitionCmdPort");
        check_reference_at_position(&server, 31, 17, "starting");
    }

    #[test]
    fn test_perform_hover() {
        let mut server = create_server_no_stdlib();
        let source = r#"
use case def TransportPassengerDef {
    action a {
        action driverGetInVehicle {
            action unlockDoor_in;
        }
    }
}

part def Test {
    use case transportPassenger : TransportPassengerDef;
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;

        let uri = open_test_document(&mut server, source);

        println!("=== Testing perform hover ===\n");

        // Print all references in the file
        println!("ALL REFERENCES:");
        let refs = server
            .workspace()
            .reference_index()
            .get_references_in_file("/test.sysml");
        for r in refs {
            println!("  {} at {:?}", r.source_qname, r.span);
        }

        let tests = [
            (11, 12, "transportPassenger (col 12)"),
            (11, 16, "transportPassenger (col 16)"),
            (12, 12, "transportPassenger (col 12)"),
        ];

        println!("\n=== Testing hover positions ===");
        for (line, col, desc) in tests {
            let pos = Position::new(line, col);

            let ref_at_pos = server
                .workspace()
                .reference_index()
                .get_full_reference_at_position(
                    "/test.sysml",
                    SysterPosition::new(line as usize, col as usize),
                );
            println!("\n({},{}) {}:", line, col, desc);
            println!("  ref_at_pos: {:?}", ref_at_pos.map(|(t, _)| t));

            let hover = server.get_hover(&uri, pos);
            match hover {
                Some(h) => {
                    println!("  HOVER: found");
                    if let async_lsp::lsp_types::HoverContents::Scalar(
                        async_lsp::lsp_types::MarkedString::String(s),
                    ) = h.contents
                    {
                        println!("  Content: {}", s.lines().next().unwrap_or(""));
                    }
                }
                None => {
                    println!("  HOVER: NOT FOUND");
                }
            }
        }
    }
}

// ============================================================
// DEBUG TESTS - WITH STDLIB
// ============================================================

mod debug_with_stdlib {
    use super::*;

    #[test]
    fn test_anon_return_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package Test {
    public import ScalarValues::*;
    
    calc def ComputeBSFC {
        in engine: Engine;
        return : Real;
    }
    
    part def Engine;
}"#;

        let uri = open_test_document(&mut server, source);

        print_references_in_test_file(&server);

        println!("\n=== HOVER TESTS on line 5 (return : Real;) ===");
        test_hover_at_columns(&server, &uri, 5, 8, 30);
    }

    #[test]
    fn test_bind_hover() {
        let mut server = create_server_with_stdlib();
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

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test");
        print_references_in_test_file(&server);

        // Try hover at various positions on line 8 (the bind line)
        println!("\n=== HOVER TESTS ===");
        test_hover_at_columns(&server, &uri, 8, 8, 50);
    }

    #[test]
    fn test_first_then_hover() {
        let mut server = create_server_with_stdlib();
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

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test");
        print_references_in_test_file(&server);

        // Try hover at various positions on line 9 (the first X then Y line)
        println!("\n=== HOVER TESTS (line 9) ===");
        test_hover_at_columns(&server, &uri, 9, 8, 55);
    }

    #[test]
    fn test_nested_public_import_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package SimpleVehicleModel {
    public import AttributeDefinitions::*;
    
    package AttributeDefinitions {
        public import ScalarValues::*;
    }
    
    calc def ComputeBSFC {
        return : Real;
    }
}"#;

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "SimpleVehicleModel");
        print_references_in_test_file(&server);

        // Try hover on "return : Real;" (line 8)
        println!("\n=== HOVER TESTS on line 8 (return : Real) ===");
        test_hover_at_columns(&server, &uri, 8, 8, 25);
    }

    #[test]
    fn test_perform_action_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package Test {
    action def ProvidePower;
    
    part def Vehicle {
        perform action providePower;
    }
}"#;

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test");
        print_references_in_test_file(&server);

        // Try hover at various positions
        println!("\n=== HOVER TESTS ===");
        test_hover_at_columns(&server, &uri, 4, 22, 40);
    }

    #[test]
    fn test_return_real_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package Test {
    private import ScalarValues::*;
    
    calc def ComputeValue {
        in x : Real;
        return : Real;
    }
}"#;

        let uri = open_test_document(&mut server, source);

        print_symbols_filtered(&server, "Test");
        print_references_in_test_file(&server);

        // Try hover on line 5 (return : Real;)
        println!("\n=== HOVER TESTS on line 5 ===");
        test_hover_at_columns(&server, &uri, 5, 8, 30);

        // Also check line 4 (in x : Real;)
        println!("\n=== HOVER TESTS on line 4 (in x : Real) ===");
        test_hover_at_columns(&server, &uri, 4, 8, 25);
    }

    #[test]
    fn test_specializes_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package Test {
    import ISQ::*;
    
    part def Vehicle {
        attribute mass :> ISQ::mass;
    }
}"#;

        let uri = open_test_document(&mut server, source);

        print_all_symbols(&server);
        print_references_in_test_file(&server);

        // Try hover at various positions on line 4
        println!("\n=== HOVER TESTS ===");
        test_hover_at_columns(&server, &uri, 4, 24, 35);

        println!("\n=== REFERENCE AT POSITION ===");
        check_reference_at_position(&server, 4, 26, "ISQ");
        check_reference_at_position(&server, 4, 30, "mass");
    }

    #[test]
    fn test_transitive_import_hover() {
        let mut server = create_server_with_stdlib();
        let source = r#"package Test {
    private import ISQ::*;
    
    calc def ComputeValue {
        in x : Real;
        return : Real;
    }
}"#;

        let uri = open_test_document(&mut server, source);

        println!("\n=== CHECKING IMPORTS ===");
        for sym in server.workspace().symbol_table().iter_symbols() {
            if (sym.qualified_name() == "ISQ" || sym.qualified_name().starts_with("ISQ::"))
                && !sym.qualified_name().contains("::")
            {
                println!("  {}", sym.qualified_name());
            }
        }

        println!("\n=== HOVER TESTS on line 4 (in x : Real) ===");
        test_hover_at_columns(&server, &uri, 4, 13, 22);

        println!("\n=== HOVER TESTS on line 5 (return : Real) ===");
        test_hover_at_columns(&server, &uri, 5, 13, 25);
    }
}

// ============================================================
// RESOLUTION TESTS
// ============================================================

mod resolution_tests {
    use super::*;

    #[test]
    fn test_real_resolves_in_calc_def() {
        let mut server = create_server_with_stdlib();
        let source = r#"package SimpleVehicleModel {
    public import Definitions::*;
    
    package Definitions {
        public import AttributeDefinitions::*;
        
        package AttributeDefinitions {
            public import ScalarValues::*;
        }
    }
    
    package VehicleAnalysis {
        calc def ComputeBSFC {
            return : Real;
        }
    }
}"#;

        let uri = open_test_document(&mut server, source);

        // Line 13 (0-indexed): "            return : Real;"
        // Real starts at column 21
        let pos = Position {
            line: 13,
            character: 21,
        };
        let hover = server.get_hover(&uri, pos);

        eprintln!("[DEBUG] Hover result: {:?}", hover);

        assert!(
            hover.is_some(),
            "Hover on 'Real' should resolve to ScalarValues::Real via transitive public imports"
        );
    }

    #[test]
    fn test_real_resolves_in_parameter() {
        let mut server = create_server_with_stdlib();
        let source = r#"package SimpleVehicleModel {
    public import Definitions::*;
    
    package Definitions {
        public import AttributeDefinitions::*;
        
        package AttributeDefinitions {
            public import ScalarValues::*;
        }
    }
    
    package VehicleAnalysis {
        calc def FuelConsumption {
            in bestFuelConsumption: Real;
        }
    }
}"#;

        let uri = open_test_document(&mut server, source);

        // Line 13: "            in bestFuelConsumption: Real;"
        // Real is at columns 36-40 (0-indexed)
        let pos = Position {
            line: 13,
            character: 36,
        };
        let hover = server.get_hover(&uri, pos);

        assert!(
            hover.is_some(),
            "Hover on 'Real' in parameter should resolve to ScalarValues::Real"
        );
    }
}
