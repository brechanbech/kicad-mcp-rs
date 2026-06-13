//! Tool implementations, one submodule per backend.
//!
//! Each submodule owns its parameter structs (which derive `JsonSchema` so rmcp can
//! advertise the tool's input schema) and an async `execute`-style function. The
//! server's `#[tool]` methods are thin delegators to these, so the server file reads
//! as an index and the real work is grouped by backend.

use rmcp::ErrorData as McpError;

#[cfg(feature = "ipc")]
pub mod board;
pub mod export;
#[cfg(feature = "files")]
pub mod project;

/// Maps an access-layer error into an MCP protocol error.
///
/// A disabled/missing backend is a client-facing "bad request" (they asked for a
/// capability this build doesn't have); everything else is an internal error.
// Takes the error by value so it reads as a point-free `.map_err(to_mcp)` at call
// sites; the body only needs a borrow, hence the allow.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn to_mcp(err: kicad_access::Error) -> McpError {
    if err.is_backend_disabled() {
        McpError::invalid_params(err.to_string(), None)
    } else {
        McpError::internal_error(err.to_string(), None)
    }
}
