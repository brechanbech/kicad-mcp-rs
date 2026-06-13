//! The headless file backend: parse KiCad documents with no KiCad process.
//!
//! Built on `kiutils-rs`, which does lossless S-expression round-trips of
//! `.kicad_pcb`, `.kicad_pro`, and library tables. This is the backend that makes a
//! GUI-free, CI-friendly server possible — the IPC API explicitly cannot open or
//! manipulate board files without KiCad running.
//!
//! The functions here are intentionally [`AsRef<Path>`]-accepting and I/O-light so
//! the parsing logic stays testable against fixtures (M-IMPL-ASREF).

use std::path::Path;

use crate::{Error, ErrorKind, Result};

/// A flat, serializable summary of a board, independent of the parser's types.
///
/// Returning our own type rather than re-exporting `kiutils-rs` internals keeps the
/// dependency from leaking through our public API (M-DONT-LEAK-TYPES).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoardSummary {
    /// Number of footprints placed on the board.
    pub footprints: usize,
    /// Number of distinct nets.
    pub nets: usize,
    /// Enabled copper layer count.
    pub copper_layers: usize,
}

/// Parses a `.kicad_pcb` and returns a high-level summary, without launching KiCad.
///
/// # Errors
/// Returns [`ErrorKind::File`] if the path can't be read or parsed.
///
/// # Examples
/// ```no_run
/// # async fn f() -> kicad_access::Result<()> {
/// let summary = kicad_access::files::summarize_board("board.kicad_pcb")?;
/// println!("{} footprints across {} nets", summary.footprints, summary.nets);
/// # Ok(()) }
/// ```
pub fn summarize_board(path: impl AsRef<Path>) -> Result<BoardSummary> {
    let path = path.as_ref();
    tracing::debug!(target: "kicad.files.parse.start", path = %path.display());

    let doc = kiutils_rs::PcbFile::read(path).map_err(|e| Error::with_source(ErrorKind::File, e))?;
    let ast = doc.ast();

    let footprints = ast.footprints.len();
    // KiCad always emits net 0 — the implicit "no-net" with an empty name — so exclude
    // it; the count is of actual nets the user defined.
    let nets = ast.nets.iter().filter(|n| n.code != Some(0)).count();
    // Copper layers are exactly the ones whose canonical name ends in `.Cu`
    // (`F.Cu`, `In1.Cu`, …, `B.Cu`); `layer_type` is unreliable for this (silk and
    // edge layers also carry a type), so key off the name.
    let copper_layers = ast
        .layers
        .iter()
        .filter(|l| l.name.as_deref().is_some_and(|n| n.ends_with(".Cu")))
        .count();

    tracing::debug!(target: "kicad.files.parse.ok", footprints, nets, copper_layers);
    Ok(BoardSummary { footprints, nets, copper_layers })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    #[test]
    fn summarizes_a_two_footprint_board() {
        let summary = summarize_board(fixture("two_resistor.kicad_pcb")).expect("parse fixture");
        assert_eq!(summary.footprints, 2);
        // 4 copper (F.Cu, In1.Cu, In2.Cu, B.Cu); F.SilkS and Edge.Cuts excluded.
        assert_eq!(summary.copper_layers, 4);
        // 3 real nets (GND, VCC, SIG); the net-0 no-net is not counted.
        assert_eq!(summary.nets, 3);
    }

    #[test]
    fn missing_file_is_a_file_error() {
        let err = summarize_board(fixture("does_not_exist.kicad_pcb")).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::File);
    }
}
