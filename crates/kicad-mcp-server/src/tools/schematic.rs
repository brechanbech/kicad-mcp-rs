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
use rmcp::model::{CallToolResult, ContentBlock};
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
    Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
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
    /// Library symbol ids defined in the sheet — the set `place_symbol` may instantiate.
    lib_symbols: Vec<String>,
    /// Paper size, e.g. `A4`.
    paper: Option<String>,
    /// Drawing-frame fields, if a title block is present.
    title_block: Option<TitleBlockInfo>,
}

/// The title-block fields surfaced in a summary (comments are omitted).
#[derive(Debug, Serialize)]
struct TitleBlockInfo {
    title: Option<String>,
    date: Option<String>,
    rev: Option<String>,
    company: Option<String>,
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
        lib_symbols: sch.lib_symbol_ids(),
        paper: sch.paper(),
        title_block: sch.title_block().map(|tb| TitleBlockInfo {
            title: tb.title,
            date: tb.date,
            rev: tb.rev,
            company: tb.company,
        }),
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

// --- import library symbol (edit) ---

/// Parameters for the `import_lib_symbol` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportLibSymbolParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Absolute path to the `.kicad_sym` library file to import from.
    pub library_path: PathBuf,
    /// Library id to import, e.g. `Device:R` or `Amplifier_Operational:OPA197xD`. The part
    /// after `:` is the symbol's name in the library file.
    pub lib_id: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of an `import_lib_symbol` call.
#[derive(Debug, Serialize)]
struct ImportResult {
    /// `false` if the definition was already present (no write needed).
    imported: bool,
    /// The sheet's placeable `lib_symbols` after the call.
    lib_symbols: Vec<String>,
    written_to: Option<PathBuf>,
}

/// Imports a symbol definition from a `.kicad_sym` library into the sheet's `lib_symbols`
/// (flattening derived symbols as KiCad does), making it placeable. Errors clearly if the
/// symbol isn't in the library.
#[allow(clippy::unused_async)]
pub async fn import_lib_symbol(params: ImportLibSymbolParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let already = sch.lib_symbol_ids().contains(&params.lib_id);

    let imported = sch
        .import_lib_symbol(&params.library_path, &params.lib_id)
        .map_err(to_mcp)?;
    if !imported && !already {
        return Err(McpError::invalid_params(
            format!(
                "symbol '{}' not found in {}",
                params.lib_id,
                params.library_path.display()
            ),
            None,
        ));
    }

    let written_to = if imported {
        Some(write_back(&sch, &params.sch_path, params.output_path)?)
    } else {
        None
    };
    json_result(&ImportResult {
        imported,
        lib_symbols: sch.lib_symbol_ids(),
        written_to,
    })
}

// --- place symbol (edit) ---

/// Parameters for the `place_symbol` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlaceSymbolParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Library id to instantiate, e.g. `Device:R`. Must already be in the sheet's
    /// `lib_symbols` (see `summarize_schematic`'s `lib_symbols` for what's available).
    pub lib_id: String,
    /// Reference for the new symbol, e.g. `R5`. Use `R?`-style if you intend to annotate.
    pub reference: String,
    /// Value, e.g. `10k`.
    pub value: String,
    /// X position (mm).
    pub x: f64,
    /// Y position (mm).
    pub y: f64,
    /// Rotation in degrees (0/90/180/270). Defaults to 0.
    #[serde(default)]
    pub angle: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of a `place_symbol` call.
#[derive(Debug, Serialize)]
struct PlaceResult {
    /// UUID of the newly placed symbol instance.
    uuid: String,
    written_to: PathBuf,
}

/// Places a new symbol instance and writes the result. Validates that `lib_id` is defined
/// in the sheet first, so the failure is a clear message rather than a silent no-op.
#[allow(clippy::unused_async)]
pub async fn place_symbol(params: PlaceSymbolParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;

    let available = sch.lib_symbol_ids();
    if !available.contains(&params.lib_id) {
        return Err(McpError::invalid_params(
            format!(
                "lib_id '{}' is not defined in this sheet's lib_symbols; available: [{}]",
                params.lib_id,
                available.join(", ")
            ),
            None,
        ));
    }

    let uuid = sch
        .place_symbol(
            &params.lib_id,
            &params.reference,
            &params.value,
            params.x,
            params.y,
            params.angle,
        )
        .map_err(to_mcp)?
        .ok_or_else(|| McpError::internal_error("symbol placement failed unexpectedly", None))?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&PlaceResult { uuid, written_to })
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

// --- connections & iteration (edit) ---

/// A newly added node (label, junction, …) and where the sheet was written.
#[derive(Debug, Serialize)]
struct AddedNode {
    uuid: String,
    written_to: PathBuf,
}

/// A node placed at a resolved point (e.g. on a pin), with that point echoed back.
#[derive(Debug, Serialize)]
struct PlacedAtPoint {
    uuid: String,
    x: f64,
    y: f64,
    written_to: PathBuf,
}

/// A symbol-level edit (move/delete) and where the sheet was written.
#[derive(Debug, Serialize)]
struct SymbolEdit {
    reference: String,
    written_to: PathBuf,
}

/// An add-method returned `None` only when the sheet has no `kicad_sch` root.
fn require_uuid(uuid: Option<String>) -> Result<String, McpError> {
    uuid.ok_or_else(|| McpError::internal_error("document has no kicad_sch root", None))
}

/// Parameters for the `label_pin` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LabelPinParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol whose pin to label, e.g. `U1`.
    pub reference: String,
    /// Pin number on that symbol, e.g. `1`.
    pub pin: String,
    /// Net name the label carries, e.g. `SDA`. Pins sharing a net name are connected.
    pub net: String,
    /// Label rotation in degrees. Defaults to 0.
    #[serde(default)]
    pub angle: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Attaches a net label to a pin's connection point — the way to connect pins by net name
/// (no wire needed). Two pins with the same net label are on the same net.
#[allow(clippy::unused_async)]
pub async fn label_pin(params: LabelPinParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let at = resolve_pin(&sch, &params.reference, &params.pin)?;
    let uuid = require_uuid(
        sch.add_label(&params.net, at.x, at.y, params.angle)
            .map_err(to_mcp)?,
    )?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&PlacedAtPoint {
        uuid,
        x: at.x,
        y: at.y,
        written_to,
    })
}

/// Parameters for the `add_label` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddLabelParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Net name the label carries.
    pub text: String,
    /// X position (mm).
    pub x: f64,
    /// Y position (mm).
    pub y: f64,
    /// Label rotation in degrees. Defaults to 0.
    #[serde(default)]
    pub angle: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Places a local net label at a point (e.g. on a wire) — for naming a net not anchored to
/// a specific pin. Use `label_pin` to label a pin directly.
#[allow(clippy::unused_async)]
pub async fn add_label(params: AddLabelParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let uuid = require_uuid(
        sch.add_label(&params.text, params.x, params.y, params.angle)
            .map_err(to_mcp)?,
    )?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&AddedNode { uuid, written_to })
}

/// Parameters for the `add_junction` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddJunctionParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// X position (mm).
    pub x: f64,
    /// Y position (mm).
    pub y: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Adds a junction dot where wires cross and should connect (a T or + intersection).
#[allow(clippy::unused_async)]
pub async fn add_junction(params: AddJunctionParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let uuid = require_uuid(sch.add_junction(params.x, params.y).map_err(to_mcp)?)?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&AddedNode { uuid, written_to })
}

/// Parameters for the `mark_no_connect` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkNoConnectParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol, e.g. `U1`.
    pub reference: String,
    /// Pin number to flag as intentionally unconnected, e.g. `7`.
    pub pin: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Flags a pin as intentionally unconnected (a no-connect), so it passes ERC.
#[allow(clippy::unused_async)]
pub async fn mark_no_connect(params: MarkNoConnectParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let at = resolve_pin(&sch, &params.reference, &params.pin)?;
    let uuid = require_uuid(sch.add_no_connect(at.x, at.y).map_err(to_mcp)?)?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&PlacedAtPoint {
        uuid,
        x: at.x,
        y: at.y,
        written_to,
    })
}

/// Parameters for the `move_symbol` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveSymbolParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol to move, e.g. `R1`.
    pub reference: String,
    /// New X position (mm).
    pub x: f64,
    /// New Y position (mm).
    pub y: f64,
    /// New rotation in degrees. Defaults to 0.
    #[serde(default)]
    pub angle: f64,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Moves a placed symbol to a new position/rotation.
#[allow(clippy::unused_async)]
pub async fn move_symbol(params: MoveSymbolParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    if !sch
        .move_symbol(&params.reference, params.x, params.y, params.angle)
        .map_err(to_mcp)?
    {
        return Err(McpError::invalid_params(
            format!("no symbol with reference {}", params.reference),
            None,
        ));
    }
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&SymbolEdit {
        reference: params.reference,
        written_to,
    })
}

/// Parameters for the `delete_symbol` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteSymbolParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Reference of the symbol to delete, e.g. `R1`.
    pub reference: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Deletes a placed symbol from the sheet.
#[allow(clippy::unused_async)]
pub async fn delete_symbol(params: DeleteSymbolParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    if !sch.delete_symbol(&params.reference).map_err(to_mcp)? {
        return Err(McpError::invalid_params(
            format!("no symbol with reference {}", params.reference),
            None,
        ));
    }
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&SymbolEdit {
        reference: params.reference,
        written_to,
    })
}

// --- page settings (edit) ---

/// Standard KiCad paper sizes accepted by `set_page_settings`.
const PAPER_SIZES: &[&str] = &[
    "A0", "A1", "A2", "A3", "A4", "A5", "A", "B", "C", "D", "E", "USLetter", "USLegal", "USLedger",
];

/// Parameters for the `set_page_settings` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetPageSettingsParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Paper size — one of A0–A5, A–E, `USLetter`, `USLegal`, `USLedger`.
    pub paper_size: String,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of a `set_page_settings` call.
#[derive(Debug, Serialize)]
struct PageResult {
    paper: String,
    written_to: PathBuf,
}

/// Sets the sheet's paper size.
#[allow(clippy::unused_async)]
pub async fn set_page_settings(params: SetPageSettingsParams) -> Result<CallToolResult, McpError> {
    if !PAPER_SIZES.contains(&params.paper_size.as_str()) {
        return Err(McpError::invalid_params(
            format!(
                "unknown paper size '{}'; expected one of [{}]",
                params.paper_size,
                PAPER_SIZES.join(", ")
            ),
            None,
        ));
    }
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    sch.set_paper(&params.paper_size).map_err(to_mcp)?;
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    json_result(&PageResult {
        paper: params.paper_size,
        written_to,
    })
}

/// Parameters for the `set_title_block` tool. Provide any subset of fields to set.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetTitleBlockParams {
    /// Absolute path to the `.kicad_sch`.
    pub sch_path: PathBuf,
    /// Drawing title.
    #[serde(default)]
    pub title: Option<String>,
    /// Date string (stored verbatim).
    #[serde(default)]
    pub date: Option<String>,
    /// Revision.
    #[serde(default)]
    pub rev: Option<String>,
    /// Company name.
    #[serde(default)]
    pub company: Option<String>,
    /// Where to write the result. Defaults to overwriting `sch_path` in place.
    #[serde(default)]
    pub output_path: Option<PathBuf>,
}

/// Outcome of a `set_title_block` call — the resulting fields and where it was written.
#[derive(Debug, Serialize)]
struct TitleBlockResult {
    title: Option<String>,
    date: Option<String>,
    rev: Option<String>,
    company: Option<String>,
    written_to: PathBuf,
}

/// Sets the given title-block fields (any subset of title/date/rev/company), creating the
/// title block if needed.
#[allow(clippy::unused_async)]
pub async fn set_title_block(params: SetTitleBlockParams) -> Result<CallToolResult, McpError> {
    let mut sch = Schematic::from_file(&params.sch_path).map_err(to_mcp)?;
    let fields = [
        ("title", &params.title),
        ("date", &params.date),
        ("rev", &params.rev),
        ("company", &params.company),
    ];
    let mut any = false;
    for (field, value) in fields {
        if let Some(value) = value {
            sch.set_title_block_field(field, value).map_err(to_mcp)?;
            any = true;
        }
    }
    if !any {
        return Err(McpError::invalid_params(
            "provide at least one of title, date, rev, company".to_owned(),
            None,
        ));
    }
    let written_to = write_back(&sch, &params.sch_path, params.output_path)?;
    let tb = sch.title_block().unwrap_or_default();
    json_result(&TitleBlockResult {
        title: tb.title,
        date: tb.date,
        rev: tb.rev,
        company: tb.company,
        written_to,
    })
}
