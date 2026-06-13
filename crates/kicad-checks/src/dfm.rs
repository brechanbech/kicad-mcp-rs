//! Design-for-manufacturability gates.
//!
//! These encode fab-house constraints (minimum trace width, drill, annular ring) as
//! gates over extracted board geometry. Each is a small [`Gate`] implementor so it can
//! be tested and run independently.
//!
//! **Clearance is deliberately not here.** A clearance check is a pairwise geometric
//! analysis over all copper — exactly what KiCad's DRC does — so reimplementing it
//! from the file would duplicate (worse) what we already get for free. Clearance
//! violations flow into the report through [`crate::drc`] instead.

use serde::Deserialize;

use crate::gates::{Gate, GateOutcome, Severity};

/// The minimal board facts the DFM gates need. Populated by the server from whichever
/// access backend produced the data; this crate never reads files itself.
#[derive(Debug, Clone, Deserialize)]
pub struct BoardGeometry {
    /// Narrowest copper trace on the board, in millimeters.
    pub min_trace_width_mm: f64,
    /// Smallest drilled hole diameter, in millimeters.
    pub min_drill_mm: f64,
    /// Smallest annular ring (`(pad − drill) / 2`) on a plated hole, in millimeters.
    pub min_annular_ring_mm: f64,
}

/// Builds a blocking-or-pass outcome for a "measured must be ≥ floor" rule.
///
/// Shared by the DFM gates, which differ only in which fact they read and the wording.
/// The `EPSILON` slack keeps a value exactly at the floor from failing on rounding.
fn floor_outcome(id: &'static str, label: &str, measured_mm: f64, floor_mm: f64) -> GateOutcome {
    if measured_mm + f64::EPSILON < floor_mm {
        GateOutcome {
            gate: id.to_owned(),
            severity: Some(Severity::Error),
            findings: vec![format!(
                "{label} {measured_mm:.3} mm is below the {floor_mm:.3} mm fab floor"
            )],
        }
    } else {
        GateOutcome::pass(id)
    }
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
        floor_outcome(
            self.id(),
            "narrowest trace",
            input.min_trace_width_mm,
            self.floor_mm,
        )
    }
}

/// Fails when any drilled hole is smaller than the fab's minimum drill.
#[derive(Debug, Clone)]
pub struct MinDrill {
    /// Fab minimum drill diameter, in millimeters.
    pub floor_mm: f64,
}

impl Gate for MinDrill {
    type Input = BoardGeometry;

    fn id(&self) -> &'static str {
        "dfm.min-drill"
    }

    fn evaluate(&self, input: &BoardGeometry) -> GateOutcome {
        floor_outcome(
            self.id(),
            "smallest drill",
            input.min_drill_mm,
            self.floor_mm,
        )
    }
}

/// Fails when any plated hole's annular ring is thinner than the fab can hold.
#[derive(Debug, Clone)]
pub struct MinAnnularRing {
    /// Fab minimum annular ring, in millimeters.
    pub floor_mm: f64,
}

impl Gate for MinAnnularRing {
    type Input = BoardGeometry;

    fn id(&self) -> &'static str {
        "dfm.min-annular-ring"
    }

    fn evaluate(&self, input: &BoardGeometry) -> GateOutcome {
        floor_outcome(
            self.id(),
            "smallest annular ring",
            input.min_annular_ring_mm,
            self.floor_mm,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Geometry comfortably above any floor the tests use; override one field per test.
    fn ok_geometry() -> BoardGeometry {
        BoardGeometry {
            min_trace_width_mm: 1.0,
            min_drill_mm: 1.0,
            min_annular_ring_mm: 1.0,
        }
    }

    #[test]
    fn min_trace_width_flags_below_and_passes_at_or_above() {
        let gate = MinTraceWidth { floor_mm: 0.15 };
        let mut geom = ok_geometry();
        geom.min_trace_width_mm = 0.10;
        assert!(gate.evaluate(&geom).is_blocking());
        geom.min_trace_width_mm = 0.20;
        assert!(!gate.evaluate(&geom).is_blocking());
    }

    #[test]
    fn min_drill_flags_below_and_passes_at_or_above() {
        let gate = MinDrill { floor_mm: 0.20 };
        let mut geom = ok_geometry();
        geom.min_drill_mm = 0.15;
        assert!(gate.evaluate(&geom).is_blocking());
        geom.min_drill_mm = 0.30;
        assert!(!gate.evaluate(&geom).is_blocking());
    }

    #[test]
    fn min_annular_ring_flags_below_and_passes_at_or_above() {
        let gate = MinAnnularRing { floor_mm: 0.13 };
        let mut geom = ok_geometry();
        geom.min_annular_ring_mm = 0.10;
        assert!(gate.evaluate(&geom).is_blocking());
        geom.min_annular_ring_mm = 0.15;
        assert!(!gate.evaluate(&geom).is_blocking());
    }

    #[test]
    fn a_gate_only_reads_its_own_fact() {
        // A thin ring must not trip the drill gate, and vice-versa.
        let mut geom = ok_geometry();
        geom.min_annular_ring_mm = 0.01;
        assert!(!MinDrill { floor_mm: 0.20 }.evaluate(&geom).is_blocking());
        assert!(
            MinAnnularRing { floor_mm: 0.13 }
                .evaluate(&geom)
                .is_blocking()
        );
    }
}
