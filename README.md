# kicad-mcp-rs

A Model Context Protocol (MCP) server for [KiCad](https://www.kicad.org/), written
in Rust. It lets an MCP client (Claude, etc.) inspect and validate KiCad PCB
designs — DRC, design-for-manufacturability gates, headless board analysis — over
the three ways Rust can reach KiCad.

> Status: **working**. All three backends are implemented and validated against
> KiCad 10.0.3 — `files` (headless board summary + geometry), `cli` (DRC plus the
> DFM quality gate, end-to-end), and `ipc` (live connection / version). `cargo check`
> is clean from the first clone (the default build pulls only well-established
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

## Tools

| Tool              | Backend | In default build | Notes                              |
|-------------------|---------|------------------|------------------------------------|
| `run_drc`         | cli     | yes              | Raw KiCad DRC JSON report          |
| `quality_gate`    | cli + checks | yes         | DRC + DFM gates → pass/fail; DFM geometry needs `--features files` |
| `summarize_board` | files   | `--features files` | Footprints/nets/layers, headless |
| `kicad_version`   | ipc     | `--features ipc` | Smoke test of the live connection  |

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

CI runs on [Codeberg CI](https://docs.codeberg.org/ci/) (Woodpecker).
[`.woodpecker.yml`](.woodpecker.yml) runs `cargo fmt --check`, clippy (`-D warnings`),
the tests, and `cargo deny` on the default and `--features files` builds — the
headless path that needs no KiCad install. The `ipc` backend is excluded from CI (it
pulls cmake/nng and can only be exercised against a live GUI); it's validated locally
with `cargo test -p kicad-access --features ipc -- --ignored live_version`. The
`quality_gate` end-to-end test runs for real when `kicad-cli` is on `PATH` and skips
otherwise, so CI stays green without a KiCad install.

Codeberg CI is **approval-gated and granted per user**: request access once by opening
an issue at [`Codeberg-CI/request-access`](https://codeberg.org/Codeberg-CI/request-access)
(link prior projects so it isn't taken for abuse), then enable this repo at
[`ci.codeberg.org/repos/add`](https://ci.codeberg.org/repos/add). Until then the
pipeline definition sits ready but does not run.

## License

MIT — see [`LICENSE.md`](LICENSE.md). Dependency and any adapted-logic notices are in
[`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md) and [`ATTRIBUTION.md`](ATTRIBUTION.md).
KiCad itself (GPLv3) is not linked; all access is arm's-length (separate process /
file parsing).
