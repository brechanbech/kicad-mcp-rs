# Third-party licenses

This project links several Rust crates and (at runtime) the KiCad application.
Their licenses are reproduced or referenced below. Generate the dependency portion
mechanically rather than by hand:

```sh
cargo install cargo-about
cargo about generate about.hbs > THIRD-PARTY-LICENSES.html
# or, for a quick text listing:
cargo install cargo-license
cargo license
```

## Direct dependencies (summary — verify with `cargo license`)

| Crate            | Role                                  | License (verify) |
|------------------|---------------------------------------|------------------|
| `rmcp`           | MCP server SDK                        | MIT              |
| `kicad-ipc-rs`   | KiCad IPC API client (feature `ipc`)  | MIT              |
| `kiutils-rs`     | KiCad S-expression files (feat `files`) | verify         |
| `tokio`          | async runtime                         | MIT              |
| `serde`/`serde_json` | (de)serialization                 | MIT OR Apache-2.0 |
| `schemars`       | JSON Schema for tool params           | MIT              |
| `anyhow`/`thiserror` | error handling                    | MIT OR Apache-2.0 |
| `tracing`*       | structured logging                    | MIT              |

\* `cargo-deny` (see `deny.toml`) enforces the permissive allow-list in CI so a
non-permissive transitive dependency fails the build instead of slipping in.

## Runtime dependency: KiCad

KiCad itself (GPLv3) is **not linked** by this project. The `ipc` backend talks to
a separate KiCad process over the IPC socket, and the `cli` backend shells out to
`kicad-cli`; the `files` backend parses file formats. All three are arm's-length
interop, so KiCad's GPL does not extend to this MIT-licensed code. (Not legal
advice.)

## Ported logic

See `ATTRIBUTION.md`. Any third-party *code* adapted into `kicad-checks` has its
upstream license reproduced here. _(none yet)_
