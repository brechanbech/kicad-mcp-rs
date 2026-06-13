//! CLI-backed tools: DRC and the quality gate.
//!
//! These are in the default build because they need only `kicad-cli` at runtime, and
//! they show the full bridge: access layer (run DRC) -> domain layer (apply gates) ->
//! MCP result.

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

use kicad_access::cli::KicadCli;
use kicad_checks::QualityReport;
use kicad_checks::dfm::{BoardGeometry, MinTraceWidth};
use kicad_checks::gates::Gate as _;

use super::to_mcp;

/// Parameters for the `run_drc` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunDrcParams {
    /// Absolute path to the `.kicad_pcb` to check.
    pub pcb_path: PathBuf,
}

/// Runs DRC and returns KiCad's raw JSON report as the tool result.
pub async fn run_drc(cli: &KicadCli, params: RunDrcParams) -> Result<CallToolResult, McpError> {
    let report = cli.run_drc(&params.pcb_path).await.map_err(to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(report)]))
}

/// Parameters for the `quality_gate` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QualityGateParams {
    /// Absolute path to the `.kicad_pcb` to gate.
    pub pcb_path: PathBuf,
    /// Fab minimum trace width in millimeters (DFM floor).
    #[serde(default = "default_trace_floor")]
    pub min_trace_width_mm: f64,
}

fn default_trace_floor() -> f64 {
    // JLCPCB standard 2-layer capability; documented so changing it is a deliberate act.
    0.127
}

/// Runs DRC then DFM gates, returning an aggregated pass/fail [`QualityReport`].
pub async fn quality_gate(
    cli: &KicadCli,
    params: QualityGateParams,
) -> Result<CallToolResult, McpError> {
    // 1. Access layer: get the machine-readable DRC report.
    let _drc_json = cli.run_drc(&params.pcb_path).await.map_err(to_mcp)?;

    // 2. Domain layer: run gates over extracted geometry.
    //    TODO(impl): extract real geometry (from the DRC report and/or the `files`
    //    backend) instead of this placeholder.
    let geometry = BoardGeometry {
        min_trace_width_mm: 0.2,
        min_drill_mm: 0.3,
    };
    let trace_gate = MinTraceWidth {
        floor_mm: params.min_trace_width_mm,
    };
    let report = QualityReport::from_outcomes(vec![trace_gate.evaluate(&geometry)]);

    // 3. MCP result.
    let json = report
        .to_json()
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
