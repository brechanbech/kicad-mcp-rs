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
// The DFM floors are consumed only by the geometry gates, which need the `files`
// backend; in a cli-only build nothing reads them, but they stay in the advertised
// tool schema. (`pcb_path` is always used, via `run_drc`.)
#[cfg_attr(not(feature = "files"), allow(dead_code))]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QualityGateParams {
    /// Absolute path to the `.kicad_pcb` to gate.
    pub pcb_path: PathBuf,
    /// Fab minimum trace width in millimeters (DFM floor).
    #[serde(default = "default_trace_floor")]
    pub min_trace_width_mm: f64,
    /// Fab minimum drill diameter in millimeters (DFM floor).
    #[serde(default = "default_drill_floor")]
    pub min_drill_mm: f64,
    /// Fab minimum annular ring in millimeters (DFM floor).
    #[serde(default = "default_annular_floor")]
    pub min_annular_ring_mm: f64,
}

// JLCPCB standard 2-layer capabilities; documented so changing one is deliberate.
fn default_trace_floor() -> f64 {
    0.127
}
fn default_drill_floor() -> f64 {
    0.2
}
fn default_annular_floor() -> f64 {
    0.13
}

/// Runs DRC then DFM gates, returning an aggregated pass/fail [`QualityReport`].
pub async fn quality_gate(
    cli: &KicadCli,
    params: QualityGateParams,
) -> Result<CallToolResult, McpError> {
    // 1. Access layer: get the machine-readable DRC report.
    let drc_json = cli.run_drc(&params.pcb_path).await.map_err(to_mcp)?;

    // 2. Domain layer: fold DRC findings, then DFM geometry gates, into one report.
    let drc = kicad_checks::DrcReport::from_json(&drc_json)
        .map_err(|e| McpError::internal_error(format!("could not parse DRC report: {e}"), None))?;
    let mut outcomes = vec![drc.into_outcome()];
    outcomes.extend(dfm_outcomes(&params)?);

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
    use kicad_checks::dfm::{BoardGeometry, MinAnnularRing, MinDrill, MinTraceWidth};
    use kicad_checks::gates::Gate as _;

    let geom = kicad_access::files::extract_geometry(&params.pcb_path).map_err(to_mcp)?;
    // An absent measurement means the board has no feature of that kind; INFINITY
    // makes the corresponding "must be ≥ floor" gate pass vacuously.
    let geometry = BoardGeometry {
        min_trace_width_mm: geom.min_trace_width_mm.unwrap_or(f64::INFINITY),
        min_drill_mm: geom.min_drill_mm.unwrap_or(f64::INFINITY),
        min_annular_ring_mm: geom.min_annular_ring_mm.unwrap_or(f64::INFINITY),
    };

    Ok(vec![
        MinTraceWidth {
            floor_mm: params.min_trace_width_mm,
        }
        .evaluate(&geometry),
        MinDrill {
            floor_mm: params.min_drill_mm,
        }
        .evaluate(&geometry),
        MinAnnularRing {
            floor_mm: params.min_annular_ring_mm,
        }
        .evaluate(&geometry),
    ])
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
