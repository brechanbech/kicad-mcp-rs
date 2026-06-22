# kicad-mcp-rs

[![CI](https://codeberg.org/brechanbech/kicad-mcp-rs/actions/workflows/ci.yml/badge.svg)](https://codeberg.org/brechanbech/kicad-mcp-rs/actions)

A Model Context Protocol (MCP) server for [KiCad](https://www.kicad.org/), written
in Rust. It lets an MCP client (Claude, etc.) inspect and validate KiCad PCB
designs — DRC, design-for-manufacturability gates, headless board analysis — and
**read and edit schematics** headlessly, over the ways Rust can reach KiCad.

> Status: **v0.2.0**. Four backends, validated against KiCad 10.0.3 — `files`
> (headless board summary + geometry), `cli` (DRC plus the DFM quality gate,
> end-to-end), `ipc` (live connection / version), and `schematic` (headless
> `.kicad_sch` read/edit — a full authoring surface: import parts from `.kicad_sym`,
> place, connect with wires/labels/power, annotate, page setup, and iterate — backed
> by [`kicad-sch-rs`](https://codeberg.org/brechanbech/kicad-sch-rs) v0.1.0). `cargo
> check` is clean from the first clone (the default build pulls only well-established
> crates); opt into the KiCad-specific backends per environment. See
> [`docs/roadmap.md`](docs/roadmap.md) for what's done and what's next.

## Layout

```
crates/
  kicad-access/   reach KiCad: ipc (kicad-ipc-rs) / files (kiutils-rs) / cli (kicad-cli)
  kicad-checks/   pure domain logic: validation gates, DFM, reports  (no I/O, no KiCad dep)
  kicad-mcp-server/  the MCP binary: rmcp wiring + tool handlers
docs/architecture.md   why it's split this way
ATTRIBUTION.md         the credit rule and reference projects
```

See [`docs/architecture.md`](docs/architecture.md) for the design rationale.

## Build & run

```sh
# Default build: CLI backend only — needs only `kicad-cli` at runtime.
cargo build
cargo run -p kicad-mcp-server          # serves MCP over stdio

# Add the headless file backend (no running KiCad needed):
cargo run -p kicad-mcp-server --features files

# Add the live IPC backend (needs the KiCad 10 GUI running with the API server
# enabled — Preferences > Plugins > IPC API):
cargo run -p kicad-mcp-server --features ipc

# Add the headless schematic read/edit backend (no running KiCad needed):
cargo run -p kicad-mcp-server --features schematic

# Everything:
cargo run -p kicad-mcp-server --features full
```

Logs go to **stderr** (stdout is the MCP transport); control verbosity with
`RUST_LOG`, e.g. `RUST_LOG=kicad_mcp_server=debug,kicad_access=debug`.

## Wiring it to a client

Point your MCP client at the binary over stdio. Example client config:

```json
{
  "mcpServers": {
    "kicad": {
      "command": "/path/to/target/release/kicad-mcp-rs",
      "args": []
    }
  }
}
```

### Protocol version

The server speaks **MCP `2025-11-25`** (the latest revision) by default and negotiates
down to `2025-06-18`, `2025-03-26`, or `2024-11-05` for older clients. This tracks the
pinned `rmcp = "1.7"` SDK: `get_info` advertises rmcp's latest known revision, so
bumping `rmcp` is what adopts future MCP versions — no handler changes needed.

## Tools

| Tool              | Backend | In default build | Notes                              |
|-------------------|---------|------------------|------------------------------------|
| `run_drc`         | cli     | yes              | Raw KiCad DRC JSON report          |
| `quality_gate`    | cli + checks | yes         | DRC + DFM gates → pass/fail; DFM geometry needs `--features files` |
| `summarize_board` | files   | `--features files` | Footprints/nets/layers, headless |
| `kicad_version`   | ipc     | `--features ipc` | Smoke test of the live connection  |
| `summarize_schematic`   | schematic | `--features schematic` | Symbols, nets, counts, placeable lib_symbols, headless |
| `annotate_schematic`    | schematic | `--features schematic` | Assign references to unannotated symbols, save |
| `import_lib_symbol`     | schematic | `--features schematic` | Import a symbol from a .kicad_sym library (flattens derived) into lib_symbols, save |
| `place_symbol`          | schematic | `--features schematic` | Place a new symbol (lib_id already in the sheet) at x/y, save |
| `set_schematic_property`| schematic | `--features schematic` | Set/insert a symbol property, save |
| `route_schematic_wire`  | schematic | `--features schematic` | Auto-route an orthogonal wire (by x/y) around symbol bodies, save |
| `list_symbol_pins`      | schematic | `--features schematic` | A symbol's pins: number, name, x/y connection point |
| `route_wire_between_pins`| schematic | `--features schematic` | Auto-route a wire between two pins (by reference + pin number), save |
| `label_pin`             | schematic | `--features schematic` | Attach a net label to a pin (connect by net name), save |
| `add_label`             | schematic | `--features schematic` | Place a local net label at x/y (e.g. on a wire), save |
| `add_junction`          | schematic | `--features schematic` | Add a junction dot at x/y where wires connect, save |
| `mark_no_connect`       | schematic | `--features schematic` | Flag a pin as intentionally unconnected (ERC), save |
| `move_symbol`           | schematic | `--features schematic` | Move a placed symbol to a new x/y/angle, save |
| `delete_symbol`         | schematic | `--features schematic` | Delete a placed symbol, save |
| `set_page_settings`     | schematic | `--features schematic` | Set the paper size, save |
| `set_title_block`       | schematic | `--features schematic` | Set title-block fields (title/date/rev/company), save |

The `schematic` tools edit the file in place unless an `output_path` is given, and
preserve every untouched byte (formatting, comments, unknown tokens) via
`kicad-sch-rs`'s lossless splice model — no running KiCad or `kicad-cli` needed.

## Runtime prerequisites

- `kicad-cli` on `PATH` (or construct `KicadCli::at(path)`) for the `cli` tools.
- For `ipc`: the KiCad 10 GUI running with the API server enabled (Preferences >
  Plugins > IPC API), with a board/project open. There is no headless `kicad-cli`
  API server in KiCad 10, so unlike `files`/`cli` this backend needs the GUI. The
  pinned `kicad-ipc-rs` targets the KiCad 10 protobuf surface — see the version
  coupling note in `docs/architecture.md`.
- On FreeBSD/macOS, `kicad-cli` is usually under `/usr/local/bin` or the app
  bundle; on FreeBSD install `cad/kicad` from ports.

## Caveats worth reading first

- **KiCad API is NNG+protobuf, not gRPC.** `kicad-ipc-rs` owns that transport; this
  project doesn't reimplement it.
- **Version coupling.** The IPC proto surface tracks the KiCad major version. Align
  your installed KiCad, `kicad-ipc-rs`, and fixtures, or you'll get decode errors at
  runtime (not compile time). See `docs/architecture.md`.
- **Dependency licenses.** `cargo deny check` enforces a permissive allow-list
  (`deny.toml`) and currently passes — the whole tree, `kiutils-rs` and
  `kicad-ipc-rs` included, fits. CI keeps it that way.

## Quality checks

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo deny check          # cargo install cargo-deny
```

A tracked pre-push hook in [`.githooks/`](.githooks) runs the same gates as CI before
every push (it skips `cargo deny` if the tool isn't installed). Enable it once per clone:

```sh
git config core.hooksPath .githooks
```

Bypass it for a work-in-progress push with `git push --no-verify`.

## Continuous integration

CI runs on Codeberg's native [Forgejo Actions](https://forgejo.org/docs/latest/user/actions/).
[`.forgejo/workflows/ci.yml`](.forgejo/workflows/ci.yml) runs `cargo fmt --check`,
clippy (`-D warnings`), the tests, and `cargo deny` on the default and `--features
files` builds — the headless path that needs no KiCad install — on Codeberg's hosted
runners. The `ipc` backend is excluded from CI (it pulls cmake/nng and can only be
exercised against a live GUI); it's validated locally with
`cargo test -p kicad-access --features ipc -- --ignored live_version`. The
`quality_gate` end-to-end test runs for real when `kicad-cli` is on `PATH` and skips
otherwise, so CI stays green without a KiCad install.

The local [`.githooks/pre-push`](.githooks) hook runs the same gates before every push.

## Security

See [`SECURITY.md`](SECURITY.md) for the trust model and how to report issues. In short:
this is a local, unauthenticated stdio server that runs with your privileges; tool
outputs are **untrusted, board-derived text** (treat them as data, not instructions —
indirect prompt injection is the client/model's concern, not something a server can
sanitize away); board paths are validated and arguments are passed without a shell; and
`unsafe` is forbidden workspace-wide.

## License

MIT — see [`LICENSE.md`](LICENSE.md). Dependency and any adapted-logic notices are in
[`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md) and [`ATTRIBUTION.md`](ATTRIBUTION.md).
KiCad itself (GPLv3) is not linked; all access is arm's-length (separate process /
file parsing).

## MCP registry

Ownership-verification token for the [MCP registry](https://registry.modelcontextprotocol.io)
(read from this crate's rendered README on crates.io):

> Registry ownership token: `mcp-name: io.github.brechanbech/kicad-mcp-rs`
