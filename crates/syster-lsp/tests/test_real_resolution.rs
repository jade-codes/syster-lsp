//! Test that Real resolves through transitive public imports

use async_lsp::lsp_types::{Position, Url};
use syster_lsp::server::LspServer;
use std::path::PathBuf;

#[test]
fn test_real_resolves_in_calc_def() {
    // Load stdlib so Real is available
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
    // Mirrors the Vehicle Example structure:
    // - SimpleVehicleModel imports Definitions::*
    // - Definitions imports AttributeDefinitions::*  
    // - AttributeDefinitions imports ScalarValues::*
    // - ScalarValues defines Real (from stdlib)
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

    server.open_document(&uri, source).expect("Should parse");
    
    // Line 14 (1-indexed) = line 13 (0-indexed): "            return : Real;"
    // "            return : Real;"
    //  0123456789012345678901234
    // Real starts at column 21
    let pos = Position { line: 13, character: 21 };
    let hover = server.get_hover(&uri, pos);
    
    eprintln!("[DEBUG] Hover result: {:?}", hover);
    
    assert!(
        hover.is_some(),
        "Hover on 'Real' should resolve to ScalarValues::Real via transitive public imports"
    );
}

#[test]
fn test_real_resolves_in_parameter() {
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("../syster-base/sysml.library");
    
    let mut server = LspServer::with_config(true, Some(stdlib_path));
    let uri = Url::parse("file:///test.sysml").unwrap();
    
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

    server.open_document(&uri, source).expect("Should parse");
    
    // Line 13: "            in bestFuelConsumption: Real;"
    // Real is at columns 36-40 (0-indexed)
    let pos = Position { line: 13, character: 36 };
    let hover = server.get_hover(&uri, pos);
    
    assert!(
        hover.is_some(),
        "Hover on 'Real' in parameter should resolve to ScalarValues::Real"
    );
}
