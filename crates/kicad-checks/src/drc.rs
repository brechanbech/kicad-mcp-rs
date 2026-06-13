//! Typed model of `kicad-cli`'s DRC JSON report, and its mapping to a gate outcome.
//!
//! `kicad-cli pcb drc --format json` emits a report with three parallel arrays of
//! findings — design-rule `violations`, `unconnected_items`, and `schematic_parity` —
//! each entry carrying a `type`, `description`, and `severity`. This module models just
//! enough of that surface to fold the findings into a single [`GateOutcome`], so DRC
//! results flow into the aggregated [`crate::QualityReport`] alongside the DFM gates.
//!
//! The model is intentionally lenient (unknown fields ignored, unknown severities
//! surfaced rather than rejected) so a `kicad-cli` minor-version bump doesn't break
//! parsing — the report schema tracks the KiCad version.

use serde::Deserialize;

use crate::gates::{GateOutcome, Severity};

/// A parsed `kicad-cli` DRC report.
#[derive(Debug, Clone, Deserialize)]
pub struct DrcReport {
    /// Design-rule violations (clearance, track width, …).
    #[serde(default)]
    pub violations: Vec<DrcFinding>,
    /// Unconnected ratsnest endpoints.
    #[serde(default)]
    pub unconnected_items: Vec<DrcFinding>,
    /// Mismatches between the schematic and the board.
    #[serde(default)]
    pub schematic_parity: Vec<DrcFinding>,
}

/// One DRC finding, shared shape across all three report categories.
#[derive(Debug, Clone, Deserialize)]
pub struct DrcFinding {
    /// KiCad's rule identifier, e.g. `clearance` (the JSON `type` field).
    #[serde(rename = "type", default)]
    pub kind: String,
    /// Human-readable description of the finding.
    #[serde(default)]
    pub description: String,
    /// How serious KiCad considers it.
    pub severity: DrcSeverity,
}

/// A DRC finding's severity, as reported by KiCad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DrcSeverity {
    /// Blocks a release.
    Error,
    /// Worth attention, non-blocking.
    Warning,
    /// Configured to be ignored.
    Ignore,
    /// Individually excluded by the user.
    Exclusion,
    /// Any severity this build doesn't recognize (forward-compat).
    #[serde(other)]
    Unknown,
}

impl DrcSeverity {
    /// Maps to the gate severity, or `None` for findings that never block or surface
    /// (ignored / user-excluded).
    ///
    /// An unrecognized severity maps to [`Severity::Warning`]: visible, but not a
    /// release block, so a future KiCad severity doesn't spuriously fail the gate.
    fn as_gate_severity(self) -> Option<Severity> {
        match self {
            DrcSeverity::Error => Some(Severity::Error),
            DrcSeverity::Warning | DrcSeverity::Unknown => Some(Severity::Warning),
            DrcSeverity::Ignore | DrcSeverity::Exclusion => None,
        }
    }
}

impl DrcReport {
    /// Parses a `kicad-cli` DRC JSON report.
    ///
    /// # Errors
    /// Returns a [`serde_json::Error`] if `json` isn't a recognizable DRC report.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Folds every reported finding into a single `drc` [`GateOutcome`].
    ///
    /// The outcome's severity is the worst across all categories (so any DRC error
    /// blocks the release gate); findings list each surfaced item as
    /// `[category/kind] description`. Ignored and excluded findings are omitted. A
    /// clean report yields a passing outcome.
    #[must_use]
    pub fn into_outcome(&self) -> GateOutcome {
        let categories = [
            ("violation", &self.violations),
            ("unconnected", &self.unconnected_items),
            ("parity", &self.schematic_parity),
        ];

        let mut severity = None;
        let mut findings = Vec::new();
        for (category, items) in categories {
            for item in items {
                let Some(sev) = item.severity.as_gate_severity() else {
                    continue;
                };
                severity = severity.max(Some(sev));
                findings.push(format!("[{category}/{}] {}", item.kind, item.description));
            }
        }

        GateOutcome {
            gate: "drc".to_owned(),
            severity,
            findings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"{
        "source": "board.kicad_pcb",
        "coordinate_units": "mm",
        "violations": [
            {
                "type": "clearance",
                "description": "Clearance violation (0.15 mm < 0.2 mm)",
                "severity": "error",
                "items": [{ "description": "Track [GND]", "pos": { "x": 1.0, "y": 2.0 } }]
            },
            {
                "type": "silk_over_copper",
                "description": "Silkscreen overlaps copper",
                "severity": "warning"
            },
            {
                "type": "courtyards_overlap",
                "description": "Excluded overlap",
                "severity": "exclusion"
            }
        ],
        "unconnected_items": [
            { "type": "unconnected_items", "description": "Net VCC unrouted", "severity": "error" }
        ],
        "schematic_parity": []
    }"#;

    #[test]
    fn parses_and_aggregates_to_worst_severity() {
        let report = DrcReport::from_json(REPORT).expect("valid report");
        let outcome = report.into_outcome();

        assert_eq!(outcome.gate, "drc");
        assert!(outcome.is_blocking()); // an error is present
        assert_eq!(outcome.severity, Some(Severity::Error));
        // 2 errors + 1 warning surface; the exclusion is dropped.
        assert_eq!(outcome.findings.len(), 3);
        assert!(
            outcome
                .findings
                .iter()
                .any(|f| f.contains("[unconnected/unconnected_items]"))
        );
        assert!(
            !outcome
                .findings
                .iter()
                .any(|f| f.contains("Excluded overlap"))
        );
    }

    #[test]
    fn clean_report_passes() {
        let outcome = DrcReport::from_json(r#"{ "violations": [] }"#)
            .expect("valid")
            .into_outcome();
        assert!(!outcome.is_blocking());
        assert_eq!(outcome.severity, None);
        assert!(outcome.findings.is_empty());
    }

    #[test]
    fn unknown_severity_is_a_nonblocking_warning() {
        let outcome = DrcReport::from_json(
            r#"{ "violations": [{ "type": "new_check", "description": "x", "severity": "fatal" }] }"#,
        )
        .expect("valid")
        .into_outcome();
        assert_eq!(outcome.severity, Some(Severity::Warning));
        assert!(!outcome.is_blocking());
    }
}
