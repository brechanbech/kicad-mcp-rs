//! Design-for-manufacturability gates.
//!
//! These encode fab-house constraints (minimum trace width, annular ring, drill
//! sizes, …) as gates over extracted board geometry. The example below shows the
//! intended shape; real rules get added as `Gate` implementors here.

use serde::Deserialize;

use crate::gates::{Gate, GateOutcome, Severity};

/// The minimal board facts a DFM gate needs. Populated by the server from whichever
/// access backend produced the data; this crate never reads files itself.
#[derive(Debug, Clone, Deserialize)]
pub struct BoardGeometry {
    /// Narrowest copper trace on the board, in millimeters.
    pub min_trace_width_mm: f64,
    /// Smallest drilled hole diameter, in millimeters.
    pub min_drill_mm: f64,
}

/// Fails when any trace is narrower than the fab's capability.
#[derive(Debug, Clone)]
pub struct MinTraceWidth {
    /// Fab capability floor, in millimeters.
    pub floor_mm: f64,
}

impl Gate for MinTraceWidth {
    type Input = BoardGeometry;

    fn id(&self) -> &'static str {
        "dfm.min-trace-width"
    }

    fn evaluate(&self, input: &BoardGeometry) -> GateOutcome {
        if input.min_trace_width_mm + f64::EPSILON < self.floor_mm {
            GateOutcome {
                gate: self.id().to_owned(),
                severity: Some(Severity::Error),
                findings: vec![format!(
                    "narrowest trace {:.3} mm is below the {:.3} mm fab floor",
                    input.min_trace_width_mm, self.floor_mm
                )],
            }
        } else {
            GateOutcome::pass(self.id())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_traces_below_floor() {
        let gate = MinTraceWidth { floor_mm: 0.15 };
        let geom = BoardGeometry {
            min_trace_width_mm: 0.10,
            min_drill_mm: 0.3,
        };
        let outcome = gate.evaluate(&geom);
        assert!(outcome.is_blocking());
    }

    #[test]
    fn passes_traces_at_or_above_floor() {
        let gate = MinTraceWidth { floor_mm: 0.15 };
        let geom = BoardGeometry {
            min_trace_width_mm: 0.20,
            min_drill_mm: 0.3,
        };
        assert!(!gate.evaluate(&geom).is_blocking());
    }
}
