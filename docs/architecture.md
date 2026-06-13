# Architecture

The workspace is three crates, split so that the one place third-party logic could
land is isolated by a crate boundary rather than a code-review habit.

```
                       ┌──────────────────────────┐
                       │     kicad-mcp-server      │  binary
                       │  rmcp wiring + tool defs   │  (anyhow, tracing)
                       └────────────┬──────────────┘
                       depends on   │   depends on
                ┌───────────────────┴───────────────────┐
                ▼                                         ▼
   ┌────────────────────────┐               ┌────────────────────────┐
   │      kicad-access       │               │      kicad-checks       │
   │ ipc / files / cli       │               │ gates / dfm / report    │
   │ (kicad-ipc-rs,          │               │ pure logic, NO I/O,     │
   │  kiutils-rs, kicad-cli) │               │ NO KiCad dependency     │
   └────────────────────────┘               └────────────────────────┘
        the "how we reach KiCad" layer            the "what's good design" layer
```

## Why three crates

`kicad-access` and `kicad-mcp-server` are wiring over public Rust crates and the
standard library. There is nothing in them to attribute. `kicad-checks` is the
domain layer — the validation gates, DFM heuristics, and report rules — and it is
the *only* crate that could ever carry logic adapted from another project. Making it
a separate crate turns the attribution question into a structural property: if you
never add a KiCad/third-party dependency to `kicad-checks` and only feed it plain
data, you always know exactly where to look. See `ATTRIBUTION.md`.

It also makes `kicad-checks` trivially testable — it takes data and returns
judgements, so its tests are pure and fast, with no KiCad install needed.

## The three access backends

KiCad offers three surfaces, and they are not interchangeable:

| Backend | Crate         | Needs KiCad running? | Scope                              |
|---------|---------------|----------------------|------------------------------------|
| `files` | `kiutils-rs`  | No                   | Parse `.kicad_pcb`/`.kicad_pro`/libs |
| `ipc`   | `kicad-ipc-rs`| Yes (GUI or api-server) | Live PCB-editor inspection/edit  |
| `cli`   | `kicad-cli`   | No (spawns the binary) | Plot, export, DRC, ERC           |

The IPC API is protobuf over an NNG/UNIX socket — **not gRPC** — so tonic does not
apply. `kicad-ipc-rs` already implements that transport; we don't reimplement it.
The IPC API cannot open or export board files without KiCad running and does no
plotting/exporting, which is precisely why `files` and `cli` exist alongside it.

## The headless angle (the "extend it" opportunity)

Because `files` parses on disk and `cli` has a headless `kicad-cli api-server` mode,
the server can run with **no KiCad GUI** at all — suitable for FreeBSD CI and agent
pipelines. That is the differentiator over a GUI-coupled design: keep as much as
possible on the `files` + `cli` path so the server is useful without a live session.

## Version coupling (read before upgrading)

The IPC protobuf surface is tied to the KiCad major version. `kicad-ipc-rs` pins a
generated proto set for a specific KiCad release. Keep three things aligned: your
installed KiCad, the `kicad-ipc-rs` version, and any board fixtures. A mismatch
surfaces as decode errors, not compile errors.
