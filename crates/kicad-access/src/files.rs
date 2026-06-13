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

    // TODO(impl): replace with the real kiutils-rs parse, e.g.
    //   let board = kiutils_rs::pcb::Board::from_file(path)
    //       .map_err(|e| Error::with_source(ErrorKind::File, e))?;
    //   Ok(BoardSummary {
    //       footprints: board.footprints.len(),
    //       nets: board.nets.len(),
    //       copper_layers: board.copper_layer_count(),
    //   })
    let _ = path;
    Err(Error::new(ErrorKind::File))
}
