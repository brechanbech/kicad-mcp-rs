//! IPC-backed tools (feature `ipc`): act on a running KiCad instance.

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, ContentBlock};

use kicad_access::ipc::Kicad;

use super::to_mcp;

/// Connects to a running KiCad and returns its version string.
pub async fn kicad_version() -> Result<CallToolResult, McpError> {
    let kicad = Kicad::connect().await.map_err(to_mcp)?;
    let version = kicad.version().await.map_err(to_mcp)?;
    Ok(CallToolResult::success(vec![ContentBlock::text(version)]))
}
