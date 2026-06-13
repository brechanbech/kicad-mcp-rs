//! Design-quality logic for KiCad projects: gates, DFM checks, and reports.
//!
//! This crate is the **domain layer**. It takes plain, already-extracted data
//! (board summaries, DRC findings) and produces structured judgements — a
//! pass/fail quality gate, DFM warnings, a report. It performs no I/O and depends
//! on no KiCad crate, so it can be unit-tested entirely from fixtures.
//!
//! # Attribution boundary
//!
//! By design this is the *only* crate in the workspace that could ever carry logic
//! adapted from another project (e.g. a Python KiCad MCP server's validation rules).
//! The access and server crates are wiring over public crates and the standard
//! library. If you adapt third-party logic, it belongs here, and it must be recorded
//! in `ATTRIBUTION.md` with a header on the source file. As of now this is all
//! original work. See the project `ATTRIBUTION.md`.

pub mod dfm;
pub mod drc;
pub mod gates;
pub mod report;

pub use drc::DrcReport;
pub use gates::{Gate, GateOutcome, Severity};
pub use report::QualityReport;
