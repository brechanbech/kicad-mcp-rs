//! Schematic tools (feature `schematic`): headless `.kicad_sch` read and edit, backed by
//! the `kicad-sch-rs` crate. No running KiCad or `kicad-cli` is required — these operate
//! directly on the file via its lossless S-expression model, so every untouched byte
//! (formatting, comments, unknown tokens) is preserved across an edit.
//!
//! Read tools return a JSON summary; edit tools write the modified schematic back (to
//! `output_path`, or over the input in place) and report what changed.

use std::path::{Path, PathBuf};

use kicad_sch_rs::{Point, Schematic};
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Maps a `kicad-sch-rs` error to an MCP error: a missing or malformed file is the
/// caller's problem (`invalid_params`); anything else is internal.
// By value so it reads as a point-free `.map_err(to_mcp)`; the body only needs a borrow.
#[allow(clippy::needless_pass_by_value)]
fn to_mcp(err: kicad_sch_rs::Error) -> McpError {
    if err.is_io() || err.is_parse() {
        McpError::invalid_params(err.to_string(), None)
    } else {
        McpError::internal_error(err.to_string(), None)
    }
}

/// Serializes a value as pretty JSON in a successful tool result.
fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Writes the schematic to `output` (or back over `input` when `output` is `None`),
/// returning the path written.
fn write_back(sch: &Schematic, input: &Path, output: Option<PathBuf>) -> Result<PathBuf, McpError> {
    let path = output.unwrap_or_else(|| input.to_path_buf());
    std::fs::write(&path, sch.to_lossless_string())
        .map_err(|e| McpError::internal_error(format!("writing {}: {e}", path.display()), None))?;
    Ok(path)
}

// --- summarize (read) ---

/// Parameters for the `summarize_schematic` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeSchematicParams {
    /// Absolute path to the `.kicad_sch` to summarize.
    pub sch_path: PathBuf,
}

/// One placed symbol in a [`SchematicSummary`].
#[derive(Debug, Serialize)]
struct SymbolSummary {
    reference: Option<String>,
    value: Option<String>,
    lib_id: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
}

/// A headless overview of a schematic's contents.
#[derive(Debug, Serialize)]
struct SchematicSummary {
    symbols: usize,
    wires: usize,
    buses: usize,
    labels: usize,
    junctions: usize,
    no_connects: usize,
    components: Vec<SymbolSummary>,
    nets: Vec<String>,
}

/// Parses the schematic from disk and returns a JSON summary of its elements.
// async to match the rmcp tool-handler delegation; the file work itself is sync.
#[allow(clippy::unused_async)]
pub async fn summarize_schematic(
    params: SummarizeSchematicParams,
) -> Result<CallToolResult, McpError> {
    let sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let symbols = sch.symbols();
    let summary = SchematicSummary {
        symbols: symbols.len(),
        wires: sch.wires().len(),
        buses: sch.buses().len(),
        labels: sch.labels().len(),
        junctions: sch.junctions().len(),
        no_connects: sch.no_connects().len(),
        components: symbols
            .iter()
            .map(|s| SymbolSummary {
                reference: s.reference.clone(),
                value: s.value.clone(),
                lib_id: s.lib_id.clone(),
                x: s.at.map(|p| p.x),
                y: s.at.map(|p| p.y),
            })
            .collect(),
        nets: sch.labels().into_iter().map(|l| l.text).collect(),
    };
    json_result(&summary)
}

// --- annotate (edit) ---

/// Parameters for the `annotate_schematic` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnnotateSchematicParams {
    /// Absolute path to the `.kicad_sch` to annotate.
    pub sch_path: PathBuf,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of an `annotate_schematic` call.
#[derive(Debug, Serialize)]
struct AnnotateResult {
    annotated: usize,
    written_to: PathBuf,
}

/// Annotates unannotated symbols (`R?` → `R1`, …) and writes the result.
#[allow(clippy::unused_async)]
pub async fn annotate_schematic(
    params: AnnotateSchematicParams,
) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let annotated = sch.annotate().map_err(to_mcp)?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&AnnotateResult {
        annotated,
        written_to,
    })
}

// --- set property (edit) ---

/// Parameters for the `set_schematic_property` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetSchematicPropertyParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol to edit, e.g. `R1`.
    pub reference: String,
    /// Property key, e.g. `Footprint`.
    pub key: String,
    /// New value. The property is inserted if the symbol does not already have it.
    pub value: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of a `set_schematic_property` call.
#[derive(Debug, Serialize)]
struct SetPropertyResult {
    /// `false` if no symbol had the given reference (the file was left untouched).
    updated: bool,
    written_to: Option<PathBuf>,
}

/// Sets (or inserts) a property on the named symbol and writes the result.
#[allow(clippy::unused_async)]
pub async fn set_schematic_property(
    params: SetSchematicPropertyParams,
) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let updated = sch
        .upsert_symbol_property(&params.reference, &params.key, &params.value)
        .map_err(to_mcp)?;
    let written_to = if updated {
        Some(write_back(&sch, &params.sch_path, params.output_path)?)
    } else {
        None
    };
    json_result(&SetPropertyResult {
        updated,
        written_to,
    })
}

// --- route wire (edit) ---

/// Parameters for the `route_schematic_wire` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RouteSchematicWireParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Start x (mm).
    pub start_x: f64,
    /// Start y (mm).
    pub start_y: f64,
    /// End x (mm).
    pub end_x: f64,
    /// End y (mm).
    pub end_y: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// A corner of a routed wire.
#[derive(Debug, Serialize)]
struct RoutePoint {
    x: f64,
    y: f64,
}

/// Outcome of a `route_schematic_wire` call.
#[derive(Debug, Serialize)]
struct RouteResult {
    /// `false` if no obstacle-free route exists (the file was left untouched).
    routed: bool,
    /// The corner points of the route, start first (empty when `routed` is `false`).
    corners: Vec<RoutePoint>,
    written_to: Option<PathBuf>,
}

/// Auto-routes an orthogonal wire between two points around symbol bodies, writing the
/// result when a route is found.
#[allow(clippy::unused_async)]
pub async fn route_schematic_wire(
    params: RouteSchematicWireParams,
) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let start = Point {
        x: params.start_x,
        y: params.start_y,
    };
    let end = Point {
        x: params.end_x,
        y: params.end_y,
    };
    let result = match sch.route_wire(start, end).map_err(to_mcp)? {
        Some(corners) => {
            let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
            RouteResult {
                routed: true,
                corners: corners
                    .into_iter()
                    .map(|p| RoutePoint { x: p.x, y: p.y })
                    .collect(),
                written_to: Some(written_to),
            }
        }
        None => RouteResult {
            routed: false,
            corners: Vec::new(),
            written_to: None,
        },
    };
    json_result(&result)
}
