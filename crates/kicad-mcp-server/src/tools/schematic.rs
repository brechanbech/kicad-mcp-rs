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
    let result = route_and_write(&mut sch, start, end, &params.sch_path, params.output_path)?;
    json_result(&result)
}

/// Routes `start` → `end`, writing the schematic back when a route is found, and builds the
/// shared [`RouteResult`]. Shared by the coordinate- and pin-addressed routing tools.
fn route_and_write(
    sch: &mut Schematic,
    start: Point,
    end: Point,
    input: &Path,
    output: Option<PathBuf>,
) -> Result<RouteResult, McpError> {
    Ok(match sch.route_wire(start, end).map_err(to_mcp)? {
        Some(corners) => RouteResult {
            routed: true,
            corners: corners
                .into_iter()
                .map(|p| RoutePoint { x: p.x, y: p.y })
                .collect(),
            written_to: Some(write_back(sch, input, output)?),
        },
        None => RouteResult {
            routed: false,
            corners: Vec::new(),
            written_to: None,
        },
    })
}

// --- pins (read + pin-addressed routing) ---

/// Parameters for the `list_symbol_pins` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSymbolPinsParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol whose pins to list, e.g. `U1`.
    pub reference: String,
}

/// One pin in a `list_symbol_pins` result — its sheet connection point (where a wire
/// attaches) is `(x, y)`.
#[derive(Debug, Serialize)]
struct PinInfo {
    number: String,
    name: String,
    x: f64,
    y: f64,
}

/// Lists a symbol's pins with their sheet connection points, for discovering the pin
/// numbers to pass to `route_wire_between_pins`.
#[allow(clippy::unused_async)]
pub async fn list_symbol_pins(params: ListSymbolPinsParams) -> Result<CallToolResult, McpError> {
    let sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    if !sch
        .symbols()
        .iter()
        .any(|s| s.reference.as_deref() == Some(&params.reference))
    {
        return Err(McpError::invalid_params(
            format!("no symbol with reference {}", params.reference),
            None,
        ));
    }
    let pins: Vec<PinInfo> = sch
        .symbol_pins(&params.reference)
        .into_iter()
        .map(|p| PinInfo {
            number: p.number,
            name: p.name,
            x: p.at.x,
            y: p.at.y,
        })
        .collect();
    json_result(&pins)
}

/// Parameters for the `route_wire_between_pins` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RouteBetweenPinsParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the start symbol, e.g. `R1`.
    pub from_reference: String,
    /// Pin number on the start symbol, e.g. `1`.
    pub from_pin: String,
    /// Reference of the end symbol, e.g. `C3`.
    pub to_reference: String,
    /// Pin number on the end symbol, e.g. `2`.
    pub to_pin: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Resolves a `(reference, pin number)` to its sheet connection point, or a clear
/// `invalid_params` error naming what was missing.
fn resolve_pin(sch: &Schematic, reference: &str, pin: &str) -> Result<Point, McpError> {
    sch.pin_position(reference, pin).ok_or_else(|| {
        McpError::invalid_params(format!("no pin {pin} on symbol {reference}"), None)
    })
}

/// Auto-routes a wire between two symbols' pins (addressed by reference + pin number),
/// writing the result when a route is found.
#[allow(clippy::unused_async)]
pub async fn route_wire_between_pins(
    params: RouteBetweenPinsParams,
) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let start = resolve_pin(&sch, &params.from_reference, &params.from_pin)?;
    let end = resolve_pin(&sch, &params.to_reference, &params.to_pin)?;
    let result = route_and_write(&mut sch, start, end, &params.sch_path, params.output_path)?;
    json_result(&result)
}
