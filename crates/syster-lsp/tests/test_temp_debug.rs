//! Focused tests for inherited member resolution

use async_lsp::lsp_types::{Position, Url};
use std::fs;
use std::path::PathBuf;
use syster_lsp::server::LspServer;

fn setup_server() -> (LspServer, Url) {
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    
    let source = fs::read_to_string(&file_path)
        .expect("Should be able to read SimpleVehicleModel.sysml");
    
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    server.open_document(&uri, &source).expect("Should parse document");
    (server, uri)
}

#[test]
fn test_driver_performs_field() {
    // Check the expression/value pattern failures
    // Line 59: if ignitionCmd.ignitionOnOff==IgnitionOnOff::on and brakePedalDepressed
    let (server, uri) = setup_server();
    let ref_index = server.workspace().reference_index();
    
    println!("=== References on line 59 (guard expression) ===");
    for col in 30..85 {
        let core_pos = syster::core::Position::new(58, col);
        if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
            println!("Col {}: target='{}' chain={:?}", col, target, ref_info.chain_context);
        }
    }
    
    // Check hover on ignitionCmd at the start
    let pos = Position { line: 58, character: 36 };
    let hover = server.get_hover(&uri, pos);
    println!("\nHover at col 36 (ignitionCmd): {:?}", hover.is_some());
    
    // Check what ignitionCmd resolves to
    let sym_table = server.workspace().symbol_table();
    let resolver = server.resolver();
    
    // Find the ignitionCmd in off_To_starting
    let ignition = sym_table.iter_symbols()
        .find(|s| s.qualified_name() == "SimpleVehicleModel::Definitions::PartDefinitions::Vehicle::vehicleStates::operatingStates::off_To_starting::ignitionCmd");
    
    if let Some(sym) = ignition {
        eprintln!("ignitionCmd: {}", sym.qualified_name());
        if let syster::semantic::symbol_table::Symbol::Usage { usage_type, .. } = sym {
            eprintln!("  usage_type: {:?}", usage_type);
            
            // Try to resolve ignitionOnOff as a member
            if let Some(resolved) = resolver.resolve_member("ignitionOnOff", sym, sym.scope_id()) {
                eprintln!("  resolve_member('ignitionOnOff') = {}", resolved.qualified_name());
            } else {
                eprintln!("  resolve_member('ignitionOnOff') = None");
            }
        }
    } else {
        eprintln!("ignitionCmd not found!");
    }
}

#[test]
fn test_resolve_direct_symbol() {
    // Basic test: can we resolve a fully qualified symbol?
    let (server, _) = setup_server();
    let resolver = server.resolver();
    
    let result = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::DriverCmdPort::driverCmd");
    assert!(result.is_some(), "Should resolve DriverCmdPort::driverCmd by fully qualified name");
}

#[test]
fn test_resolve_supertype_from_scope() {
    // Can we resolve 'DriverCmdPort' from HandPort's scope?
    // HandPort is in the same package as DriverCmdPort
    let (server, _) = setup_server();
    let resolver = server.resolver();
    
    let handport = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::HandPort")
        .expect("HandPort should exist");
    
    let result = resolver.resolve_in_scope("DriverCmdPort", handport.scope_id());
    assert!(result.is_some(), "Should resolve 'DriverCmdPort' from HandPort's scope (same package)");
    assert_eq!(result.unwrap().qualified_name(), 
               "SimpleVehicleModel::Definitions::PortDefinitions::DriverCmdPort");
}

#[test]
fn test_resolve_inherited_member_from_type_body_scope() {
    // Can we resolve 'driverCmd' from inside HandPort's body?
    // HandPort :> DriverCmdPort, so driverCmd should be inherited
    // Note: We need to be in HandPort's BODY scope, not the package scope where HandPort is declared
    let (server, _) = setup_server();
    let resolver = server.resolver();
    
    // ignitionCmd is declared inside HandPort's body, so its scope_id is HandPort's body scope
    let ignition = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::HandPort::ignitionCmd")
        .expect("ignitionCmd should exist");
    
    // The body scope of HandPort is the scope where ignitionCmd is declared
    let handport_body_scope = ignition.scope_id();
    
    // driverCmd is defined in DriverCmdPort, HandPort specializes DriverCmdPort
    let result = resolver.resolve_in_scope("driverCmd", handport_body_scope);
    assert!(result.is_some(), 
            "Should resolve 'driverCmd' from HandPort's body scope via inheritance from DriverCmdPort");
    assert_eq!(result.unwrap().qualified_name(),
               "SimpleVehicleModel::Definitions::PortDefinitions::DriverCmdPort::driverCmd");
}

#[test]
fn test_resolve_inherited_member_from_nested_scope() {
    // Can we resolve 'driverCmd' from ignitionCmd's scope?
    // ignitionCmd is inside HandPort, so should also find driverCmd via inheritance
    let (server, _) = setup_server();
    let resolver = server.resolver();
    
    let ignition = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::HandPort::ignitionCmd")
        .expect("ignitionCmd should exist");
    
    let result = resolver.resolve_in_scope("driverCmd", ignition.scope_id());
    assert!(result.is_some(), 
            "Should resolve 'driverCmd' from ignitionCmd's scope (nested inside HandPort which inherits it)");
    assert_eq!(result.unwrap().qualified_name(),
               "SimpleVehicleModel::Definitions::PortDefinitions::DriverCmdPort::driverCmd");
}

#[test]
fn test_hover_on_subsets_target() {
    // Hover on 'driverCmd' in: out item ignitionCmd:IgnitionCmd subsets driverCmd;
    // Line 278: out item ignitionCmd:IgnitionCmd subsets driverCmd;
    let (server, uri) = setup_server();
    
    let pos = Position { line: 277, character: 60 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), "Hover on 'driverCmd' (subsets target) should return content");
    
    let content = format!("{:?}", hover.unwrap().contents);
    assert!(content.contains("driverCmd"), "Hover should mention 'driverCmd'");
}

#[test]
fn test_goto_definition_on_subsets_target() {
    // Go-to-definition on 'driverCmd' in: out item ignitionCmd:IgnitionCmd subsets driverCmd;
    let (server, uri) = setup_server();
    
    let pos = Position { line: 277, character: 60 };
    let def = server.get_definition(&uri, pos);
    
    assert!(def.is_some(), "Go-to-definition on 'driverCmd' should return a location");
}

#[test]
fn test_scope_hierarchy() {
    // Debug test to understand scope relationships
    let (server, _) = setup_server();
    let resolver = server.resolver();
    
    let handport = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::HandPort")
        .expect("HandPort should exist");
    let ignition = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::HandPort::ignitionCmd")
        .expect("ignitionCmd should exist");
    let drivercmd = resolver.resolve("SimpleVehicleModel::Definitions::PortDefinitions::DriverCmdPort::driverCmd")
        .expect("driverCmd should exist");
    
    println!("HandPort declared in scope_id: {}", handport.scope_id());
    println!("ignitionCmd declared in scope_id: {}", ignition.scope_id());
    println!("driverCmd declared in scope_id: {}", drivercmd.scope_id());
    
    // The key insight: ignitionCmd.scope_id() is NOT HandPort.scope_id()
    // ignitionCmd is declared in HandPort's BODY scope, not the package scope
    assert_ne!(ignition.scope_id(), handport.scope_id(), 
               "ignitionCmd should be in HandPort's body scope, not the package scope");
}

// ============================================================
// Tests for specializes (:>) resolution
// ============================================================

#[test]
fn test_specializes_redefinition_resolution() {
    // Line 656: requirement drivePowerOuputRequirement :>> drivePowerOutputRequirement
    // The target 'drivePowerOutputRequirement' should resolve
    let (server, uri) = setup_server();
    
    // Column for drivePowerOutputRequirement on line 656
    // "requirement drivePowerOuputRequirement :>> drivePowerOutputRequirement{"
    //                                            ^--- starts around col 52
    let pos = Position { line: 655, character: 55 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'drivePowerOutputRequirement' (redefinition target) should return content");
}

#[test]
fn test_specializes_interface_resolution() {
    // Line 969: interface wheelFastenerInterface1 :> wheelFastenerInterface
    // The target 'wheelFastenerInterface' should resolve
    let (server, uri) = setup_server();
    
    let pos = Position { line: 968, character: 55 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'wheelFastenerInterface' (specialization target) should return content");
}

// ============================================================
// Tests for first/then transition resolution
// ============================================================

#[test]
fn test_first_transition_source() {
    // Line 1423: first driverGetInVehicle then join1;
    // 'driverGetInVehicle' should resolve to the action defined earlier
    let (server, uri) = setup_server();
    
    // "first driverGetInVehicle then join1;"
    //       ^--- col ~22
    let pos = Position { line: 1422, character: 25 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'driverGetInVehicle' (first transition source) should return content");
}

#[test]
fn test_then_transition_target() {
    // Line 1423: first driverGetInVehicle then join1;
    // 'join1' should resolve to the join defined at line 1413
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    
    println!("\n=== Checking join1 resolution ===");
    
    let possible_names = [
        "join1",
        "SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1::join1",
    ];
    
    for name in &possible_names {
        let result = resolver.resolve(name);
        println!("resolver.resolve('{}') = {:?}", name, result.map(|s| s.qualified_name()));
    }
    
    let pos = Position { line: 1422, character: 45 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'join1' (then transition target) should return content");
}

// ============================================================
// Tests for accept action resolution
// ============================================================

#[test]
fn test_accept_via_port() {
    // Line 58: accept ignitionCmd:IgnitionCmd via ignitionCmdPort
    // 'ignitionCmdPort' is a port reference
    let (server, uri) = setup_server();
    
    // "accept ignitionCmd:IgnitionCmd via ignitionCmdPort"
    //                                     ^--- starts around col 52
    let pos = Position { line: 57, character: 55 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'ignitionCmdPort' (via target in accept) should return content");
}

// ============================================================
// Tests for inherited output parameter resolution
// ============================================================

#[test]
fn test_out_param_in_do_action() {
    // Line 78: out temp;
    // Inside: do senseTemperature { out temp; }
    // 'temp' should resolve to SenseTemperature::temp (inherited from action type)
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    let sym_table = server.workspace().symbol_table();
    
    println!("\n=== Checking 'temp' resolution ===");
    
    // Check if SenseTemperature::temp exists
    let sense_temp = resolver.resolve("SimpleVehicleModel::Definitions::ActionDefinitions::SenseTemperature::temp");
    println!("SenseTemperature::temp: {:?}", sense_temp.map(|s| s.qualified_name()));
    
    // Check reference at the position
    let ref_index = server.workspace().reference_index();
    // Line 79 (0-indexed: 78): "                            out temp;"
    // List all references around this line
    println!("\n=== References around line 76-81 ===");
    for line in 76..82 {
        for col in 0..60 {
            let core_pos = syster::core::Position::new(line, col);
            if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
                println!("Line {}, col {}: target='{}' scope_id={:?}", line, col, target, ref_info.scope_id);
                break; // Found one, skip rest of columns
            }
        }
    }
    
    // Check what senseTemperature is and if it creates a scope
    let sense_temp_usage = resolver.resolve("SimpleVehicleModel::Definitions::PartDefinitions::Vehicle::senseTemperature");
    if let Some(st) = sense_temp_usage {
        println!("\n=== senseTemperature usage ===");
        println!("Qualified name: {}", st.qualified_name());
        println!("Symbol type: {:?}", st);
        println!("Scope ID: {:?}", st.scope_id());
    }
    
    let core_pos = syster::core::Position::new(77, 32);
    if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
        println!("Reference: target='{}' scope_id={:?}", target, ref_info.scope_id);
        
        if let Some(scope_id) = ref_info.scope_id {
            // Walk the scope chain to understand the hierarchy
            println!("\n=== Scope chain from {} ===", scope_id);
            let scopes = sym_table.scopes();
            let mut current_scope = Some(scope_id);
            while let Some(sid) = current_scope {
                if sid < scopes.len() {
                    let scope = &scopes[sid];
                    println!("Scope {}", sid);
                    // Print symbols in this scope
                    let symbols: Vec<_> = sym_table.iter_symbols()
                        .filter(|s| s.scope_id() == sid)
                        .map(|s| format!("{} ({})", s.name(), s.qualified_name()))
                        .collect();
                    if !symbols.is_empty() {
                        println!("  Symbols: {:?}", symbols);
                    }
                    current_scope = scope.parent;
                } else {
                    break;
                }
            }
            
            // Try resolve_in_scope
            let result = resolver.resolve_in_scope(target, scope_id);
            println!("\nresolve_in_scope('{}', {}) = {:?}", target, scope_id, result.map(|s| s.qualified_name()));
        }
    } else {
        println!("No reference found!");
    }
    
    // "                            out temp;"
    //                                  ^--- col ~32
    let pos = Position { line: 78, character: 32 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'temp' (inherited out parameter) should return content");
}

// ============================================================
// Tests for specializes interface resolution
// ============================================================

#[test]
fn test_specializes_interface_member() {
    // Line 969: interface wheelFastenerInterface1 :> wheelFastenerInterface
    // 'wheelFastenerInterface' is defined in WheelHubInterface (line 333)
    // The usage is inside wheelHubInterface which is typed as WheelHubInterface
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    
    println!("\n=== Checking wheelFastenerInterface resolution ===");
    
    // Check if the definition exists
    let interface_def = resolver.resolve("SimpleVehicleModel::Definitions::InterfaceDefinitions::WheelHubInterface::wheelFastenerInterface");
    println!("wheelFastenerInterface definition: {:?}", interface_def.map(|s| s.qualified_name()));
    
    // Check the parent wheelHubInterface usage
    let wheel_hub_iface = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::WheelHubAssemblies::wheelHubAssy2::wheelHubInterface");
    if let Some(whi) = wheel_hub_iface {
        println!("wheelHubInterface usage: {}", whi.qualified_name());
        println!("  Symbol type: {:?}", whi);
    }
    
    // Check what reference is indexed at this position
    let ref_index = server.workspace().reference_index();
    let core_pos = syster::core::Position::new(968, 65);
    if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
        println!("Reference: target='{}' scope_id={:?}", target, ref_info.scope_id);
        
        // Try resolve_in_scope
        if let Some(scope_id) = ref_info.scope_id {
            let result = resolver.resolve_in_scope(target, scope_id);
            println!("resolve_in_scope('{}', {}) = {:?}", target, scope_id, result.map(|s| s.qualified_name()));
        }
    } else {
        println!("No reference found at position!");
    }
    
    // "interface wheelFastenerInterface1 :> wheelFastenerInterface"
    //                                       ^--- col ~61
    let pos = Position { line: 968, character: 65 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'wheelFastenerInterface' (specialization target inside interface usage) should return content");
}

// ============================================================
// Tests for subject specializes resolution
// ============================================================

#[test]
fn test_subject_specializes_resolution() {
    // Line 1166: subject vehicleAlternatives[2]:>vehicle_b;
    // 'vehicle_b' is defined in VehicleConfiguration_b::PartsTree
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    
    println!("\n=== Checking vehicle_b resolution ===");
    
    // Check if vehicle_b is defined
    let vehicle_b = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b");
    println!("vehicle_b definition: {:?}", vehicle_b.map(|s| s.qualified_name()));
    
    // Check reference at position
    let ref_index = server.workspace().reference_index();
    let sym_table = server.workspace().symbol_table();
    let core_pos = syster::core::Position::new(1165, 50);
    if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
        println!("Reference: target='{}' scope_id={:?}", target, ref_info.scope_id);
        
        if let Some(scope_id) = ref_info.scope_id {
            // Check imports at scope 5195
            println!("\n=== Imports at scope 5195 ===");
            let imports = sym_table.get_scope_imports(5195);
            for imp in &imports {
                println!("Import: {:?}", imp);
            }
            
            let resolved_imports = sym_table.get_resolved_imports(5195);
            println!("\n=== Resolved imports at scope 5195 ({} total) ===", resolved_imports.len());
            for ri in resolved_imports.iter() {
                println!("  raw='{}' resolved='{}' recursive={}", ri.raw_path, ri.resolved_path, ri.is_recursive);
            }
            
            let result = resolver.resolve_in_scope(target, scope_id);
            println!("\nresolve_in_scope('{}', {}) = {:?}", target, scope_id, result.map(|s| s.qualified_name()));
        }
    } else {
        println!("No reference found!");
    }
    
    // "subject vehicleAlternatives[2]:>vehicle_b;"
    //                                  ^--- col ~48
    let pos = Position { line: 1165, character: 50 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'vehicle_b' (subject specialization target) should return content");
}

// ============================================================
// Tests for feature chain via port resolution
// ============================================================

#[test]
fn test_feature_chain_via_port() {
    // Line 755: action turnVehicleOn send ignitionCmd via driver.p1{
    // 'driver' resolves to part driver : Driver
    // 'p1' is a port on Driver
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    
    println!("\n=== Checking driver.p1 resolution ===");
    
    // Check if Driver::p1 exists
    let driver_def = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::Driver::p1");
    println!("Driver::p1 definition: {:?}", driver_def.map(|s| s.qualified_name()));
    
    // Check the driver usage
    let driver_usage = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::driver");
    if let Some(d) = driver_usage {
        println!("driver usage: {}", d.qualified_name());
        println!("  Symbol: {:?}", d);
    }
    
    // Try to resolve the chain driver.p1
    if let Some(driver) = driver_usage {
        let p1 = resolver.resolve_member("p1", driver, driver.scope_id());
        println!("resolve_member('p1', driver) = {:?}", p1.map(|s| s.qualified_name()));
    }
    
    // Check reference at the position for 'p1'
    // "action turnVehicleOn send ignitionCmd via driver.p1{"
    //                                              ^--- col 77
    let ref_index = server.workspace().reference_index();
    
    // List all references around this line
    println!("\n=== References around line 754-756 ===");
    for line in 754..757 {
        for col in 0..90 {
            let core_pos = syster::core::Position::new(line, col);
            if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
                println!("Line {}, col {}: target='{}' chain={:?}", line, col, target, ref_info.chain_context);
                break; // Found one, skip rest of columns
            }
        }
    }
    
    let core_pos = syster::core::Position::new(754, 77);
    if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
        println!("\nAt position: target='{}' scope_id={:?} chain={:?}", target, ref_info.scope_id, ref_info.chain_context);
    } else {
        println!("\nNo reference found at position!");
    }
    
    let pos = Position { line: 754, character: 77 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(hover.is_some(), 
            "Hover on 'p1' (via driver.p1 feature chain) should return content");
}

// ============================================================
// Tests for join/fork node resolution
// ============================================================

#[test]
fn test_join_node_declaration() {
    // Line 1413: join join1;
    // This declares a join node named 'join1'
    let (server, _uri) = setup_server();
    let resolver = server.resolver();
    
    println!("\n=== Checking join1 declaration ===");
    
    // Check if join1 exists as a symbol
    let join1 = resolver.resolve("SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1::join1");
    println!("join1 symbol: {:?}", join1.map(|s| format!("{} ({})", s.name(), s.qualified_name())));
    
    // Also try searching for any symbol named 'join1'
    let sym_table = server.workspace().symbol_table();
    let matching: Vec<_> = sym_table.iter_symbols()
        .filter(|s| s.name() == "join1")
        .map(|s| s.qualified_name().to_string())
        .collect();
    println!("Symbols named 'join1': {:?}", matching);
    
    // Try different patterns
    let fork1 = resolver.resolve("SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1::fork1");
    println!("fork1 symbol: {:?}", fork1.map(|s| s.qualified_name()));
    
    // Check if trigger (accept action) exists
    let trigger = resolver.resolve("SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1::trigger");
    println!("trigger symbol: {:?}", trigger.map(|s| s.qualified_name()));

    // Check if driverGetInVehicle exists (action in same use case)
    let driver = resolver.resolve("SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1::driverGetInVehicle");
    println!("driverGetInVehicle symbol: {:?}", driver.map(|s| s.qualified_name()));
    
    // What symbols ARE in transportPassenger_1?
    let tp1 = resolver.resolve("SimpleVehicleModel::MissionContext::TransportPassengerScenario::transportPassenger_1");
    if let Some(tp1_sym) = tp1 {
        println!("\ntransportPassenger_1 scope_id: {:?}", tp1_sym.scope_id());
        let symbols_in_tp1: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.scope_id() == tp1_sym.scope_id())
            .map(|s| s.name().to_string())
            .collect();
        println!("Symbols in transportPassenger_1 ({} total): {:?}", symbols_in_tp1.len(), symbols_in_tp1);
        
        // Get the body scope - symbols defined IN the use case should be in a child scope
        let scopes = sym_table.scopes();
        if tp1_sym.scope_id() < scopes.len() {
            // Find child scopes
            for (i, scope) in scopes.iter().enumerate() {
                if scope.parent == Some(tp1_sym.scope_id()) {
                    let child_symbols: Vec<_> = sym_table.iter_symbols()
                        .filter(|s| s.scope_id() == i)
                        .map(|s| s.name().to_string())
                        .collect();
                    println!("Child scope {} symbols: {:?}", i, child_symbols);
                }
            }
        }
    }
    
    assert!(join1.is_some(), "join1 should exist as a symbol");
}

// ============================================================
// Tests for perform action chain resolution
// ============================================================

#[test]
fn test_perform_action_chain() {
    // Line 771: perform startVehicle.turnVehicleOn;
    // 'startVehicle' should resolve, then 'turnVehicleOn' is a member of startVehicle's type
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    let ref_index = server.workspace().reference_index();
    
    println!("\n=== Checking perform startVehicle.turnVehicleOn ===");
    
    // First check if startVehicle exists
    let start_vehicle = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::startVehicle");
    if let Some(sv) = start_vehicle {
        println!("startVehicle: {}", sv.qualified_name());
        println!("  Symbol: {:?}", sv);
        
        // Check if turnVehicleOn exists as a direct child
        let turn_qname = format!("{}::turnVehicleOn", sv.qualified_name());
        let turn_direct = resolver.resolve(&turn_qname);
        println!("Direct child {}::turnVehicleOn: {:?}", sv.qualified_name(), turn_direct.map(|s| s.qualified_name()));
        
        // List all children of startVehicle
        let sym_table = server.workspace().symbol_table();
        let children: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.qualified_name().starts_with(&format!("{}::", sv.qualified_name())))
            .map(|s| s.qualified_name().to_string())
            .collect();
        println!("Children of startVehicle: {:?}", children);
        
        // Try to resolve turnVehicleOn as a member
        let turn = resolver.resolve_member("turnVehicleOn", sv, sv.scope_id());
        println!("resolve_member('turnVehicleOn', startVehicle) = {:?}", turn.map(|s| s.qualified_name()));
    } else {
        println!("startVehicle not found!");
        
        // Try finding it in different locations
        let sym_table = server.workspace().symbol_table();
        let matching: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.name() == "startVehicle")
            .map(|s| s.qualified_name().to_string())
            .collect();
        println!("Symbols named 'startVehicle': {:?}", matching);
    }
    
    // Check what references exist around line 771
    println!("\n=== References around line 770-773 ===");
    for line in 770..774 {
        for col in 0..80 {
            let core_pos = syster::core::Position::new(line, col);
            if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
                println!("Line {}, col {}: target='{}' chain={:?}", line, col, target, ref_info.chain_context);
            }
        }
    }
    
    // Specifically check 'turnVehicleOn' at col 49
    let core_pos = syster::core::Position::new(770, 49);
    if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
        println!("\nAt turnVehicleOn position: target='{}' scope_id={:?} chain={:?}", 
                 target, ref_info.scope_id, ref_info.chain_context);
    } else {
        println!("\nNo reference found at turnVehicleOn position!");
    }
    
    // "perform startVehicle.turnVehicleOn;"
    //                       ^--- col 49
    let pos = Position { line: 770, character: 49 };
    let hover = server.get_hover(&uri, pos);
    
    println!("\nHover result: {:?}", hover);
    
    assert!(hover.is_some(), 
            "Hover on 'turnVehicleOn' (perform action chain) should return content");
}

#[test]
fn test_message_endpoint_feature_chain() {
    // Line 781: message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    // The 'driver.turnVehicleOn' refers to an action that 'driver' performs
    // The 'turnVehicleOn' is NOT a direct child of 'driver', but driver performs startVehicle.turnVehicleOn
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    let ref_index = server.workspace().reference_index();
    let sym_table = server.workspace().symbol_table();
    
    println!("\n=== Checking message endpoint driver.turnVehicleOn ===");
    
    // Check what 'driver' is
    let driver = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::driver");
    if let Some(d) = driver {
        println!("driver: {}", d.qualified_name());
        println!("  Symbol: {:?}", d);
        
        // List children of driver
        let children: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.qualified_name().starts_with(&format!("{}::", d.qualified_name())))
            .map(|s| format!("{} -> {:?}", s.qualified_name(), s))
            .collect();
        println!("Children of driver:");
        for c in &children {
            println!("  {}", c);
        }
        
        // Try resolve_member
        let turn = resolver.resolve_member("turnVehicleOn", d, d.scope_id());
        println!("resolve_member('turnVehicleOn', driver) = {:?}", turn.map(|s| s.qualified_name()));
    }
    
    // Also check driver::startVehicle
    let driver_start = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::driver::startVehicle");
    if let Some(ds) = driver_start {
        println!("\ndriver::startVehicle: {}", ds.qualified_name());
        println!("  Symbol: {:?}", ds);
        
        // What's in subsets/redefines?
        println!("  subsets: {:?}", ds.subsets());
        println!("  redefines: {:?}", ds.redefines());
    }
    
    // Check references at line 781
    println!("\n=== References around line 781 ===");
    for col in 60..110 {
        let core_pos = syster::core::Position::new(780, col);
        if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
            println!("Col {}: target='{}' chain={:?}", col, target, ref_info.chain_context);
        }
    }
    
    // "message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;"
    //                                              ^--- col 71
    let pos = Position { line: 780, character: 72 };
    let hover = server.get_hover(&uri, pos);
    
    println!("\nHover on turnVehicleOn: {:?}", hover);
}

#[test]
fn test_message_source_event() {
    // Line 829: event sendSensedSpeed.sourceEvent;
    // 'sendSensedSpeed' is a message, 'sourceEvent' is inherited from Message type
    let (server, uri) = setup_server();
    let resolver = server.resolver();
    let ref_index = server.workspace().reference_index();
    
    println!("\n=== Checking sendSensedSpeed.sourceEvent ===");
    
    // Check Line 782: message of es:EngineStatus from vehicle.sendStatus to driver.trigger2;
    println!("=== Checking vehicle.sendStatus pattern (Line 782) ===");
    let sym_table = server.workspace().symbol_table();
    let ref_index = server.workspace().reference_index();
    
    // Find the vehicle part in part0
    let vehicle = sym_table.iter_symbols()
        .find(|s| s.qualified_name() == "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::vehicle");
    
    if let Some(v) = vehicle {
        println!("vehicle: {} / scope={}", v.qualified_name(), v.scope_id());
        
        // Find ALL children of vehicle (any depth) - show full details
        let prefix = format!("{}::", v.qualified_name());
        let all_children: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.qualified_name().starts_with(&prefix))
            .map(|s| format!("  {} / subsets={:?}", s.qualified_name(), s.subsets()))
            .collect();
        println!("All vehicle descendants:\n{}", all_children.join("\n"));
    }
    
    // Check the driver part too
    let driver = sym_table.iter_symbols()
        .find(|s| s.qualified_name() == "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::Sequence::part0::driver");
    if let Some(d) = driver {
        let prefix = format!("{}::", d.qualified_name());
        let all_children: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.qualified_name().starts_with(&prefix))
            .map(|s| format!("  {} / subsets={:?}", s.qualified_name(), s.subsets()))
            .collect();
        println!("\ndriver descendants:\n{}", all_children.join("\n"));
    }
    
    // Find sendSensedSpeed - use the full path from the reference
    let send = resolver.resolve("SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::DiscreteInteractions::CruiseControl2::vehicle_b::sendSensedSpeed");
    if let Some(s) = send {
        println!("sendSensedSpeed: {}", s.qualified_name());
        println!("  Symbol: {:?}", s);
        
        // Try resolve_member for sourceEvent
        let source_event = resolver.resolve_member("sourceEvent", s, s.scope_id());
        println!("resolve_member('sourceEvent', sendSensedSpeed) = {:?}", source_event.map(|sym| sym.qualified_name()));
        
        // Also check what Message looks like
        let msg = resolver.resolve("Flows::Message");
        if let Some(m) = msg {
            println!("\nMessage def: {}", m.qualified_name());
            println!("  Symbol: {:?}", m);
            
            // Check if sourceEvent is in Message
            let se = resolver.resolve("Flows::Message::sourceEvent");
            println!("Flows::Message::sourceEvent: {:?}", se.map(|sym| sym.qualified_name()));
        }
        
        // Check what SensedSpeed looks like
        let ss_type = resolver.resolve("SensedSpeed");
        println!("\nSensedSpeed (unqualified): {:?}", ss_type.map(|sym| format!("{} / {:?}", sym.qualified_name(), sym)));
        
        // Try looking it up by qualified name in VehicleModel
        let sym_table = server.workspace().symbol_table();
        let sensed_speeds: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.name() == "SensedSpeed")
            .map(|s| format!("{} / specializes={:?}", s.qualified_name(), s.specializes()))
            .collect();
        println!("All SensedSpeed symbols: {:?}", sensed_speeds);
    } else {
        println!("sendSensedSpeed not found!");
        
        // Try to find it with different paths
        let sym_table = server.workspace().symbol_table();
        let matching: Vec<_> = sym_table.iter_symbols()
            .filter(|s| s.name() == "sendSensedSpeed")
            .map(|s| s.qualified_name().to_string())
            .collect();
        println!("Symbols named 'sendSensedSpeed': {:?}", matching);
    }
    
    // Check references at the position
    println!("\n=== References around line 829 ===");
    for col in 50..70 {
        let core_pos = syster::core::Position::new(828, col);
        if let Some((target, ref_info)) = ref_index.get_full_reference_at_position("/test.sysml", core_pos) {
            println!("Col {}: target='{}' chain={:?}", col, target, ref_info.chain_context);
        }
    }
    
    // "event sendSensedSpeed.sourceEvent;"
    //                        ^--- col 54
    let pos = Position { line: 828, character: 55 };
    let hover = server.get_hover(&uri, pos);
    
    println!("\nHover on sourceEvent: {:?}", hover);
}
