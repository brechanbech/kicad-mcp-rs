# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-14

First working release — an MCP server for inspecting and validating KiCad PCB designs,
validated end-to-end against KiCad 10.0.3.

### Added

- **MCP server** over stdio (`rmcp` 1.7), speaking MCP `2025-11-25` and negotiating down
  to `2025-06-18` / `2025-03-26` / `2024-11-05` for older clients.
- **Three-crate workspace**: `kicad-access` (KiCad backends), `kicad-checks` (pure
  validation logic — no I/O, no KiCad dependency), `kicad-mcp-server` (rmcp wiring).
- **Tools**: `run_drc` and `quality_gate` (default/`cli` build), `summarize_board`
  (`--features files`), `kicad_version` (`--features ipc`).
- **`files` backend** (`kiutils-rs`, headless): board summary (footprints, nets, copper
  layers) and fab geometry (min trace width, drill, annular ring).
- **`cli` backend** (`kicad-cli`): DRC (JSON report) and Gerber export.
- **`ipc` backend** (`kicad-ipc-rs`): live connection to a running KiCad 10 GUI and a
  version query.
- **`kicad-checks`**: DFM gates (`MinTraceWidth`, `MinDrill`, `MinAnnularRing`), a typed
  DRC-report model that folds findings into gate outcomes, and an aggregated pass/fail
  `QualityReport`.

### Security

- `unsafe` forbidden workspace-wide; client-supplied board paths validated before they
  reach `kicad-cli`; subprocess arguments passed without a shell. See [`SECURITY.md`](SECURITY.md)
  for the trust model and the prompt-injection boundary.

### Infrastructure

- Forgejo Actions CI and a matching local pre-push hook (fmt, clippy `-D warnings`,
  tests, `cargo deny`); licenses and advisories enforced by `cargo deny`.

[0.1.0]: https://codeberg.org/brechanbech/kicad-mcp-rs/releases/tag/v0.1.0
