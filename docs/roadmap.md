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
- **6 — CI.** Codeberg/Woodpecker pipeline (fmt / clippy / test / deny on default and
  `--features files`), plus a `kicad-cli`-gated end-to-end `quality_gate` test.

## What's next

Nothing is blocking; these are directions, roughly by value.

- **Activate Codeberg CI** — a one-time enable of the repo at
  [ci.codeberg.org](https://ci.codeberg.org) so `.woodpecker.yml` actually runs.
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
