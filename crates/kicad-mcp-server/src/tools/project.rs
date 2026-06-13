//! File-backed tools (feature `files`): headless analysis with no running KiCad.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

use super::to_mcp;

/// Parameters for the `summarize_board` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeBoardParams {
    /// Absolute path to the `.kicad_pcb` to summarize.
    pub pcb_path: PathBuf,
}

/// Parses the board from disk and returns a JSON summary.
// async to match the rmcp tool-handler delegation; the file parse itself is sync.
#[allow(clippy::unused_async)]
pub async fn summarize_board(params: SummarizeBoardParams) -> Result<CallToolResult, McpError> {
    let summary = kicad_access::files::summarize_board(&params.pcb_path).map_err(to_mcp)?;
    let json = serde_json::to_string_pretty(&summary)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
