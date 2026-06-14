# Roadmap

Where the skeleton goes from here. The structure, wiring, and types are in place
(`cargo check` is clean across the `default` / `files` / `ipc` / `full` configs and
the `kicad-checks` tests pass); what remains is filling the `TODO(impl)` bodies with
real KiCad calls and growing the domain layer.

The ordering is deliberate: the headless `files` + `cli` path is the project's
differentiator (it works with no KiCad GUI — see [`architecture.md`](architecture.md)),
so it leads. The live `ipc` backend, which needs a running KiCad and carries the
version-coupling caveat, comes later.

## 0. Quality baseline

Clear the decks so the gates the README advertises actually pass.

- `cargo fmt --all` (reformats some original author style, e.g. `kicad-access/cli.rs`).
- Clear the pedantic clippy warnings: missing doc backticks throughout, and the
  numeric-cast `allow`s around `mm_to_nm` / `nm_to_mm` in `kicad-access/src/lib.rs`.
- Wire `cargo deny check` (config already in `deny.toml`) and confirm the dependency
  licenses fit the allow-list — including `kiutils-rs` and `kicad-ipc-rs`.

Its own commit, since the formatting touches files unrelated to feature work.

## 1. `files::summarize_board` — the headless path

Replace the stub in `kicad-access/src/files.rs` with the real `kiutils-rs` parse:
footprint count, distinct nets, enabled copper layers. Add a small fixture
`.kicad_pcb` and a parse test. Highest value — it needs nothing installed and is the
basis for CI / agent pipelines.

## 2. Real geometry for `quality_gate`

Kill the placeholder `BoardGeometry { min_trace_width_mm: 0.2, min_drill_mm: 0.3 }`
in `kicad-mcp-server/src/tools/export.rs`. Extract real min-trace / min-drill via the
`files` backend (and/or the DRC report) and feed the gate actual board data. This is
the bridge that makes the quality gate mean something end-to-end.

## 3. DRC report → typed model in `kicad-checks`

`run_drc` currently returns KiCad's raw JSON untouched. Add a typed parse of
`kicad-cli`'s DRC report plus a gate that turns violations into `GateOutcome`s, so DRC
findings flow into the aggregated pass/fail `QualityReport` alongside the DFM gates.

> Attribution note: if any DRC/DFM rule logic is adapted from another project, it lands
> in `kicad-checks` and gets recorded per [`ATTRIBUTION.md`](../ATTRIBUTION.md). The
> access and server crates stay pure wiring.

## 4. More DFM gates

Grow the gate set in `kicad-checks/src/dfm.rs`: minimum annular ring, minimum drill,
clearance. Each is a small `Gate` implementor with unit tests — the framework
(`Gate` / `GateOutcome` / `Severity` / `QualityReport`) is already there to populate.

## 5. `ipc::Kicad` — the live backend

Wire the real `kicad-ipc-rs` client for `connect` / `version` in
`kicad-access/src/ipc.rs`. Needs the running KiCad 10 GUI with the API server enabled
(no headless API server exists in KiCad 10) and is subject to the proto/version
coupling documented in [`architecture.md`](architecture.md).

## 6. CI

A Codeberg pipeline running `fmt` / `clippy` / `test` / `deny` on the default build and
`--features files`, so the headless path stays green without a KiCad install.

---

**Suggested first pass: 0 → 1 → 2.** That gets the headless quality gate working
end-to-end against a real board, which is the project's core promise.
