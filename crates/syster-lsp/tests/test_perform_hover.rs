// Test hover on perform action usage
use syster_lsp::server::LspServer;
use async_lsp::lsp_types::{Position, Url};
use tracing_subscriber;

#[test]
fn test_perform_hover() {
    // Enable tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();

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

    let mut server = LspServer::new();
    let uri = Url::parse("file:///test.sysml").unwrap();
    let _ = server.open_document(&uri, source);
    
    println!("=== Testing perform hover ===\n");
    
    // Print all references in the file
    println!("ALL REFERENCES:");
    let refs = server.workspace().reference_index().get_references_in_file("/test.sysml");
    for r in refs {
        println!("  {} at {:?}", r.source_qname, r.span);
    }
    
    // Line 11 (0-indexed): "    perform transportPassenger;"
    //                       0123456789012345678901234567890
    //                                  ^ col 12 = start of transportPassenger
    
    // Check what's at line 11
    println!("\nSource line 11: '{}'", source.lines().nth(11).unwrap_or("N/A"));
    println!("Source line 12: '{}'", source.lines().nth(12).unwrap_or("N/A"));
    
    let tests = [
        (11, 12, "transportPassenger (col 12)"),
        (11, 16, "transportPassenger (col 16)"),
        (12, 12, "transportPassenger (col 12)"),
    ];
    
    println!("\n=== Testing hover positions ===");
    for (line, col, desc) in tests {
        let pos = Position::new(line, col);
        
        // Check what reference is at this position
        let ref_at_pos = server.workspace().reference_index()
            .get_full_reference_at_position("/test.sysml", syster::core::Position::new(line as usize, col as usize));
        println!("\n({},{}) {}:", line, col, desc);
        println!("  ref_at_pos: {:?}", ref_at_pos.map(|(t,_)| t));
        
        let hover = server.get_hover(&uri, pos);
        match hover {
            Some(h) => {
                println!("  HOVER: found");
                if let async_lsp::lsp_types::HoverContents::Scalar(async_lsp::lsp_types::MarkedString::String(s)) = h.contents {
                    println!("  Content: {}", s.lines().next().unwrap_or(""));
                }
            }
            None => {
                println!("  HOVER: NOT FOUND");
            }
        }
    }
}
