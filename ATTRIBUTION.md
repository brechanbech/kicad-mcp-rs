# Attribution

This file is the single place where this project records what it owes to others,
and the decision procedure for when a credit is required.

## The rule this project follows

Credit tracks contribution. If another project's *code or specific logic* ends up
here (translated, adapted, or copied), it gets credited. If this project only took
an *idea* — "an MCP server for KiCad with validation gates" — and the
implementation was written independently against public APIs, no credit is owed,
and adding one would falsely imply involvement that did not happen.

The honest test: **did writing a given module require looking back at someone
else's source to remember how it worked?** If yes, that's their expression, and it
gets attributed. If the module was written from the public crate/API docs alone,
it's independent work.

## Where attribution could land

By design, the only crate that can carry ported third-party *logic* is
[`kicad-checks`](crates/kicad-checks) — the validation gates, DFM heuristics, and
report rules. The `kicad-access` and `kicad-mcp-server` crates are wiring over
public Rust crates (see `THIRD-PARTY-LICENSES.md`) and the standard library, so the
attribution question never arises there. Keep it that way: if you find yourself
pulling logic from another project, it belongs in `kicad-checks`, and it gets a
note here and a header in the relevant source file.

## Reference projects (looked at, not derived from)

These shaped the *concept* and were studied during design. As of project start,
**no code from them is present** in this repository. Update this section the moment
that stops being true.

- `oaslananka/kicad-mcp` (MIT) — a Python MCP server for KiCad. Studied for its
  tool surface and validation-gate concept. This project is an **independent Rust
  reimplementation**, not a port and not affiliated with or endorsed by its author.

## If you do port something

1. Add the source to the list below with the file(s) it touched.
2. Put a short header on the derived source file:
   `// Logic adapted from <project> (<license>): <one line on what>`.
3. Make sure the upstream license text is in `THIRD-PARTY-LICENSES.md`.

### Derived logic actually in this repo

_(none yet)_
