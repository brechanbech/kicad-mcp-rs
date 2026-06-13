//! Unified access layer over the three ways to reach KiCad.
//!
//! KiCad exposes three distinct surfaces, with different capabilities and runtime
//! requirements. This crate wraps each behind a small, uniform API so the MCP server
//! never has to care which one a given operation uses:
//!
//! - [`ipc`] — the official IPC API (protobuf over an NNG/UNIX socket) via the
//!   `kicad-ipc-rs` crate. Requires a **running** KiCad instance (or a
//!   `kicad-cli api-server`). PCB-editor scope. Feature `ipc`.
//! - [`files`] — headless, KiCad-free parsing of `.kicad_pcb` / `.kicad_pro` /
//!   library tables via `kiutils-rs`. The only backend that works with no KiCad
//!   process at all — the basis for CI / agent pipelines. Feature `files`.
//! - [`cli`] — a thin wrapper over the `kicad-cli` binary for plot / export / DRC /
//!   ERC, which the IPC API does not provide. Feature `cli` (default).
//!
//! # When to use which
//!
//! Reach for [`files`] first for anything that can be answered from the project on
//! disk (it needs nothing else installed and is the cheapest). Use [`ipc`] when you
//! need to act on a board the user has open in a live session. Use [`cli`] for
//! production outputs (Gerbers, drill, BOM, PDF) and rule checks.
//!
//! # Errors
//!
//! Every backend returns [`Error`], a canonical error carrying an [`ErrorKind`] and
//! the originating cause. Prefer the `is_*` helpers over matching the kind directly.

mod error;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "files")]
pub mod files;
#[cfg(feature = "ipc")]
pub mod ipc;

pub use error::{Error, ErrorKind, Result};

/// KiCad's internal API works in nanometers; convert at the boundary.
///
/// Documented because it's a silent footgun: the IPC API reports and accepts
/// positions in nanometers (M-DOCUMENTED-MAGIC).
pub const NM_PER_MM: i64 = 1_000_000;

/// Converts a millimeter value to KiCad-internal nanometers.
// The casts are the point of the boundary conversion: board coordinates are far
// within f64's exact-integer range and i64's range, so precision/truncation here is
// the intended rounding, not a lossy surprise.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn mm_to_nm(mm: f64) -> i64 {
    // M-PANIC-IS-STOP: rounding, not error signalling — callers pass finite mm.
    (mm * NM_PER_MM as f64).round() as i64
}

/// Converts KiCad-internal nanometers to millimeters.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn nm_to_mm(nm: i64) -> f64 {
    nm as f64 / NM_PER_MM as f64
}
