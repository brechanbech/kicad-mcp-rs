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
use kicad_checks::gates::GateOutcome;

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
    // Consumed only by the geometry gate, which needs the `files` backend; in a
    // cli-only build nothing reads it, but it stays in the advertised tool schema.
    #[cfg_attr(not(feature = "files"), allow(dead_code))]
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
    // 1. Access layer: get the machine-readable DRC report. Parsing its violations
    //    into gate outcomes is a later step; running it here surfaces spawn/exec
    //    failures early and reserves the report for that work.
    let _drc_json = cli.run_drc(&params.pcb_path).await.map_err(to_mcp)?;

    // 2. Domain layer: DFM gates over geometry read from the board file.
    let outcomes = dfm_outcomes(&params)?;

    // 3. MCP result.
    let report = QualityReport::from_outcomes(outcomes);
    let json = report
        .to_json()
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// DFM gate outcomes from real board geometry, read via the headless `files` backend.
#[cfg(feature = "files")]
fn dfm_outcomes(params: &QualityGateParams) -> Result<Vec<GateOutcome>, McpError> {
    use kicad_checks::dfm::{BoardGeometry, MinTraceWidth};
    use kicad_checks::gates::Gate as _;

    let geom = kicad_access::files::extract_geometry(&params.pcb_path).map_err(to_mcp)?;

    let mut outcomes = Vec::new();
    if let Some(min_trace_width_mm) = geom.min_trace_width_mm {
        let geometry = BoardGeometry {
            min_trace_width_mm,
            // No min-drill gate exists yet; a board with no drills must not fail one.
            min_drill_mm: geom.min_drill_mm.unwrap_or(f64::INFINITY),
        };
        let gate = MinTraceWidth {
            floor_mm: params.min_trace_width_mm,
        };
        outcomes.push(gate.evaluate(&geometry));
    }
    Ok(outcomes)
}

/// Without the `files` backend the board can't be parsed, so the geometry gates are
/// reported as skipped (informational) rather than silently passing.
// Matches the `files` variant's fallible signature, which genuinely returns `Err`.
#[cfg(not(feature = "files"))]
#[allow(clippy::unnecessary_wraps)]
fn dfm_outcomes(_params: &QualityGateParams) -> Result<Vec<GateOutcome>, McpError> {
    use kicad_checks::gates::Severity;

    Ok(vec![GateOutcome {
        gate: "dfm.geometry".to_owned(),
        severity: Some(Severity::Info),
        findings: vec![
            "DFM geometry gates need the headless board parser; rebuild with --features files"
                .to_owned(),
        ],
    }])
}
