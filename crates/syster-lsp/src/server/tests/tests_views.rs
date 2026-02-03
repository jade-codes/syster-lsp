//! Tests for SysML Views endpoint
//!
//! These tests verify that the syster/getSysMLViews LSP endpoint correctly
//! discovers and returns standard SysML v2 views from the standard library.

use crate::server::tests::test_helpers::{LspServerTestExt, create_server_with_stdlib};
use crate::server::views::GetSysMLViewsParams;
use async_lsp::lsp_types::Url;
use syster::hir::SymbolKind;

#[test]
fn test_stdlib_loads_view_definitions() {
    let mut server = create_server_with_stdlib();

    // Check that stdlib is loaded
    assert!(
        server.has_stdlib_loaded(),
        "Standard library should be loaded"
    );

    // Print symbol stats
    println!("Total symbols: {}", server.symbol_count());
    println!("Total files: {}", server.loaded_file_count());

    // Find all ViewDef symbols
    let view_defs = server.find_symbols(|s| matches!(s.kind, SymbolKind::ViewDef));

    println!("\nViewDef symbols found: {}", view_defs.len());
    for view in &view_defs {
        println!(
            "  - name: '{}', qualified_name: '{}'",
            view.name, view.qualified_name
        );
    }

    // We expect to find the standard view definitions
    assert!(
        !view_defs.is_empty(),
        "Should find ViewDef symbols from stdlib"
    );
}

#[test]
#[ignore = "Standard views require ViewDef symbols which may vary with syster-base version"]
fn test_get_sysml_views_returns_standard_views() {
    let mut server = create_server_with_stdlib();

    // Call the views endpoint
    let params = GetSysMLViewsParams {
        view_name: None,
        uri: None,
    };

    let result = server.get_sysml_views(&params);

    println!("Views returned: {}", result.views.len());
    for view in &result.views {
        println!(
            "  - name: '{}', qualified_name: '{}'",
            view.name, view.qualified_name
        );
    }

    // We expect the 8 standard views
    assert!(!result.views.is_empty(), "Should return standard views");

    // Check for specific expected views
    let view_names: Vec<&str> = result.views.iter().map(|v| v.name.as_str()).collect();
    println!("\nView names: {:?}", view_names);

    // These are the expected display names (from short names gv, iv, afv, stv, sv, gev, grv, bv)
    let expected = [
        "General View",
        "Interconnection View (IBD)",
        "Action Flow View",
        "State Transition View",
        "Sequence View",
        "Geometry View",
        "Grid View",
        "Browser View",
    ];

    for expected_name in &expected {
        assert!(
            view_names.contains(expected_name),
            "Should contain '{}' view, got: {:?}",
            expected_name,
            view_names
        );
    }
}

#[test]
fn test_view_definitions_have_correct_qualified_names() {
    let mut server = create_server_with_stdlib();

    // Find all ViewDef symbols
    let view_defs = server.find_symbols(|s| matches!(s.kind, SymbolKind::ViewDef));

    // Print what we actually have
    println!("ViewDef symbols:");
    for view in &view_defs {
        println!("  qname='{}', name='{}'", view.qualified_name, view.name);
    }

    // Check if they're in StandardViewDefinitions package
    let std_views: Vec<_> = view_defs
        .iter()
        .filter(|v| v.qualified_name.contains("StandardViewDefinitions"))
        .collect();

    println!("\nStandard ViewDefs: {}", std_views.len());
    for view in &std_views {
        println!("  qname='{}', name='{}'", view.qualified_name, view.name);
    }

    assert!(
        !std_views.is_empty(),
        "Should have ViewDefs in StandardViewDefinitions"
    );
}

#[test]
fn test_view_def_symbol_names() {
    let mut server = create_server_with_stdlib();

    // Find GeneralView specifically - try different name patterns
    let gv_by_name = server.find_symbol("GeneralView");
    let gv_by_short = server.find_symbol("gv");
    let gv_by_qname = server.find_symbol_qualified("StandardViewDefinitions::GeneralView");
    let gv_by_short_qname = server.find_symbol_qualified("StandardViewDefinitions::gv");

    println!("Finding GeneralView:");
    println!(
        "  by name 'GeneralView': {:?}",
        gv_by_name.as_ref().map(|s| &s.qualified_name)
    );
    println!(
        "  by name 'gv': {:?}",
        gv_by_short.as_ref().map(|s| &s.qualified_name)
    );
    println!(
        "  by qname 'StandardViewDefinitions::GeneralView': {:?}",
        gv_by_qname.as_ref().map(|s| &s.qualified_name)
    );
    println!(
        "  by qname 'StandardViewDefinitions::gv': {:?}",
        gv_by_short_qname.as_ref().map(|s| &s.qualified_name)
    );

    // At least one should work
    assert!(
        gv_by_name.is_some()
            || gv_by_short.is_some()
            || gv_by_qname.is_some()
            || gv_by_short_qname.is_some(),
        "Should be able to find GeneralView by some name"
    );
}

#[test]
fn test_interconnection_view_filtering() {
    let mut server = create_server_with_stdlib();

    // Open a document with interconnections (parts + connections)
    let uri = Url::parse("file:///test/Interconnections.sysml").unwrap();
    let content = r#"
package CargoDrone {
    package 'Problem Space' {
        // Item types (what flows)
        item def Control;
        item def Indication;
        item def Command;

        // System of Interest
        part def 'Cargo Drone' {
            in commandIn : Command;
            out stateOut : Indication;
        }

        // External Systems / Actors
        part 'Drone Operation System' {
            in controlFromOperator : Control;
            out indicationToOperator : Indication;
            out commandToDrone : Command;
            in stateFromDrone : Indication;
        }

        part 'Drone Safety Operator' {
            out controlOut : Control;
            in indicationIn : Indication;
        }

        // Context Structure (Interconnection View)
        part 'Cargo Drone System Context' {
            part drone : 'Cargo Drone';
            part ops : 'Drone Operation System';
            part safety : 'Drone Safety Operator';

            // Safety Operator <-> Operation System
            connection connect safety.controlOut to ops.controlFromOperator;
            connection connect ops.indicationToOperator to safety.indicationIn;

            // Operation System <-> Drone
            connection connect ops.commandToDrone to drone.commandIn;
            connection connect drone.stateOut to ops.stateFromDrone;
        }
    }
}
"#;

    server.open_document(&uri, content).unwrap();

    // Debug: print all symbols from this file
    println!("\n=== All symbols in document ===");
    let all_symbols = server.all_symbols();
    for sym in &all_symbols {
        if sym.qualified_name.contains("CargoDrone") || sym.qualified_name.contains("Problem Space")
        {
            println!("  {} (kind={:?})", sym.qualified_name, sym.kind);
        }
    }

    // Now apply InterconnectionView
    let params = GetSysMLViewsParams {
        view_name: Some("StandardViewDefinitions::iv".to_string()),
        uri: Some(uri.to_string()),
    };

    let result = server.get_sysml_views(&params);

    println!("\n=== InterconnectionView applied ===");
    println!(
        "Visible symbols count: {:?}",
        result.visible_symbols.as_ref().map(|v| v.len())
    );

    if let Some(visible) = &result.visible_symbols {
        println!("Visible symbols:");
        for sym in visible {
            println!("  {}", sym);
        }
    }

    // The interconnection view should show:
    // - Parts (drone, ops, safety)
    // - Connections
    // - Ports (commandIn, stateOut, etc.)
    let visible = result.visible_symbols.expect("Should have visible symbols");

    println!("\nExpected: parts, connections, ports");
    println!("Got {} symbols", visible.len());

    // Should have at least the parts and connections
    assert!(
        !visible.is_empty(),
        "InterconnectionView should return visible symbols for a file with connections"
    );
}

#[test]
fn test_apply_view_implementation() {
    let mut server = create_server_with_stdlib();

    // Open a simple file
    let uri = Url::parse("file:///test/simple.sysml").unwrap();
    let content = r#"
part def Vehicle {
    part engine;
    part wheels;
}
part car : Vehicle;
"#;

    server.open_document(&uri, content).unwrap();

    // Get all views first
    let params = GetSysMLViewsParams {
        view_name: None,
        uri: None,
    };
    let result = server.get_sysml_views(&params);
    println!("Available views: {}", result.views.len());
    for v in &result.views {
        println!("  - {} ({})", v.name, v.qualified_name);
    }

    // Apply GeneralView
    let params = GetSysMLViewsParams {
        view_name: Some("StandardViewDefinitions::gv".to_string()),
        uri: Some(uri.to_string()),
    };
    let result = server.get_sysml_views(&params);

    println!("\n=== GeneralView applied ===");
    println!("Visible symbols: {:?}", result.visible_symbols);
}
