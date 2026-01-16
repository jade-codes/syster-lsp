//! Test helpers for LspServer tests

use crate::server::LspServer;

/// Create an LspServer without stdlib (fast, for most unit tests)
pub fn create_server() -> LspServer {
    LspServer::with_config(false, None)
}
