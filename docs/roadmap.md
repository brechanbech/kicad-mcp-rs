# Status & roadmap

The initial build-out (steps 0–6 below) is **done**: all three backends are
implemented and validated against KiCad 10.0.3, and the headless `files` + `cli` path
runs the quality gate end-to-end. `cargo check`, clippy (`-D warnings`), the tests,
and `cargo deny` are clean; CI guards the headless path (see [`../README.md`](../README.md)).

The ordering was deliberate: the headless `files` + `cli` path is the project's
differentiator (it works with no KiCad GUI — see [`architecture.md`](architecture.md)),
so it led. The live `ipc` backend, which needs the running GUI, came last.

## Done

- **0 — Quality baseline.** fmt, clippy (pedantic, `-D warnings` in CI), a
  `doc-valid-idents` allow-list, and `cargo deny` (passes clean over the whole tree).
- **1 — `files::summarize_board`.** Real `kiutils-rs` parse — footprints, nets
  (excluding KiCad's net-0 no-net), copper layers — with a fixture board and tests.
- **2 — Real geometry for `quality_gate`.** `files::extract_geometry` feeds the DFM
  gates actual min trace / drill / annular-ring values; no more placeholders.
- **3 — DRC report → gate outcome.** `kicad_checks::drc` parses `kicad-cli`'s JSON
  (pinned to a real KiCad 10 report) and folds findings into the `QualityReport`.
- **4 — DFM gates.** `MinTraceWidth`, `MinDrill`, `MinAnnularRing` over a shared
  helper. Clearance is intentionally left to DRC, not reimplemented.
- **5 — Live `ipc` backend.** `ipc::Kicad` over `kicad-ipc-rs` (`connect` / `version`),
  validated live against a running KiCad 10 GUI via the `#[ignore]d` `live_version` test.
- **6 — CI.** Codeberg Forgejo Actions pipeline ([`.forgejo/workflows/ci.yml`](../.forgejo/workflows/ci.yml):
  fmt / clippy / test / deny on default and `--features files`), plus a
  `kicad-cli`-gated end-to-end `quality_gate` test, mirrored by a local pre-push hook.

## Schematic backend (feature `schematic`)

Headless `.kicad_sch` read/edit, backed by
[`kicad-sch-rs`](https://codeberg.org/brechanbech/kicad-sch-rs) (no running KiCad /
`kicad-cli`). Tools, all validated against real KiCad 10 output:

- **Read** — `summarize_schematic` (symbols, nets, counts, placeable `lib_symbols`),
  `list_symbol_pins` (number/name/connection point).
- **Build** — `import_lib_symbol` (from a `.kicad_sym`, flattening derived `extends`
  symbols as KiCad does), `place_symbol`, `set_schematic_property`.
- **Connect** — `route_schematic_wire` / `route_wire_between_pins` (orthogonal A\*),
  `label_pin` (net label on a pin), `add_label`, `add_junction`, `mark_no_connect`.
- **Iterate** — `move_symbol`, `delete_symbol`.
- **Bulk/annotate** — `annotate_schematic`, plus bulk property edits in the crate.

Known holds (primitives exist in `kicad-sch-rs`; not yet exposed / built):

- **Delete-by-point** — `delete_wire` / `delete_junction` by coordinate, and
  `delete_label` by net name, exist in the crate but aren't MCP tools yet (awkward to
  drive by coords; `delete_label` would be trivial to add).
- **Global labels** — `add_label` emits *local* labels only; fine for single-sheet
  boards plus power symbols, but global labels are needed for multi-sheet named nets.
- **Title block / page settings** — paper size and title/rev/company/date are not yet
  read or editable (the "drawing frame"). Planned as Tier 2 toward a taggable v0.1.
- **Connectivity readback** — computed nets (which pins share a wire/label) aren't
  inferred; only declared labels are surfaced.

## What's next

The major forward plan — project setup, schematic editing, and PCB inspection & edits,
plus the native Rust `.kicad_sch` editor those depend on — is in
[`expansion-plan.md`](expansion-plan.md). Smaller near-term items:

- **Schematic Tier 2 / 3** — title block + page settings, then tag a coherent v0.1 of
  `kicad-sch-rs` + `kicad-mcp-rs` and validate with a real board end-to-end. See the
  holds above.

- **Confirm CI is green** — Forgejo Actions runs on push automatically; if it doesn't
  trigger, enable Actions in the repo's Settings.
- **More gates** — board-outline / edge clearance, silkscreen-over-pad, courtyard
  presence, net-count sanity. The `Gate` framework is there to populate.
- **More tools** — expose `summarize_board` geometry, ERC, and the `cli` export paths
  (Gerbers / drill / BOM / PDF — `KicadCli::export_gerbers` already exists) as MCP tools.
- **Richer IPC** — read live board items / selection beyond `version`, for acting on a
  board the user has open.
- **A clean-board fixture** — to complement the deliberately-failing one and assert a
  *passing* end-to-end report.
- **`spawn_blocking` for parsing** — move the synchronous `files` parse off the async
  executor if board sizes grow (currently the tool handler is async-by-contract).
