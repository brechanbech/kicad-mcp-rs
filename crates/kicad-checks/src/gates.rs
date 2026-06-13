//! The validation-gate framework.
//!
//! A [`Gate`] is one independent check against design data. Gates produce a
//! [`GateOutcome`]; a release gate is the aggregate of running many of them. Keeping
//! each gate a small object (rather than one giant function) makes them
//! individually testable and lets the server expose "run gate X" as a tool.

use serde::Serialize;

/// How serious a gate finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational; never blocks a release.
    Info,
    /// Should be looked at, but not blocking by default.
    Warning,
    /// Blocks a release gate.
    Error,
}

/// The result of running a single [`Gate`].
#[derive(Debug, Clone, Serialize)]
pub struct GateOutcome {
    /// Stable identifier of the gate that produced this (e.g. `dfm.min-trace-width`).
    pub gate: String,
    /// Worst severity found by this gate. `None` means the gate passed clean.
    pub severity: Option<Severity>,
    /// Human-readable findings.
    pub findings: Vec<String>,
}

impl GateOutcome {
    /// A clean pass for the gate `id`.
    #[must_use]
    pub fn pass(id: impl Into<String>) -> Self {
        Self {
            gate: id.into(),
            severity: None,
            findings: Vec::new(),
        }
    }

    /// True if this outcome would block a release (any `Error`-severity finding).
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.severity == Some(Severity::Error)
    }
}

/// One independent design check.
///
/// Implementors are cheap, stateless judgements over `Input`. The associated
/// `Input` is whatever extracted data the gate needs — a board summary, a parsed
/// DRC report, etc. — so gates stay decoupled from how that data was obtained.
pub trait Gate {
    /// The extracted data this gate inspects.
    type Input;

    /// Stable identifier, e.g. `dfm.min-annular-ring`.
    ///
    /// `'static` because a gate's id is a fixed constant, not derived from `&self`.
    fn id(&self) -> &'static str;

    /// Runs the check.
    fn evaluate(&self, input: &Self::Input) -> GateOutcome;
}
