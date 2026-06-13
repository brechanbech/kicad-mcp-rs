//! Aggregating gate outcomes into a single release report.

use serde::Serialize;

use crate::gates::{GateOutcome, Severity};

/// The result of running a set of gates against a design.
#[derive(Debug, Clone, Serialize)]
pub struct QualityReport {
    /// True only if no gate produced a blocking (`Error`) finding.
    pub passed: bool,
    /// Every gate's outcome, in the order they were run.
    pub outcomes: Vec<GateOutcome>,
}

impl QualityReport {
    /// Builds a report from gate outcomes, deciding pass/fail from their severities.
    #[must_use]
    pub fn from_outcomes(outcomes: Vec<GateOutcome>) -> Self {
        let passed = !outcomes.iter().any(GateOutcome::is_blocking);
        Self { passed, outcomes }
    }

    /// The worst severity across all gates, if any.
    #[must_use]
    pub fn worst_severity(&self) -> Option<Severity> {
        self.outcomes.iter().filter_map(|o| o.severity).max()
    }

    /// Serializes to pretty JSON for returning as an MCP tool result.
    ///
    /// # Errors
    /// Returns a [`serde_json::Error`] only if a contained type fails to serialize,
    /// which for these plain structs indicates a programming bug.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}
