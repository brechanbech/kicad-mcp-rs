# Security

## Trust model

`kicad-mcp-rs` is a local MCP server spoken over stdio. It runs as a subprocess of an
MCP client (an LLM agent) with **your** user privileges, and it shells out to
`kicad-cli` and reads board files you point it at. It is not a network service and does
not authenticate callers — the client is trusted to the extent your OS user is.

## Untrusted tool output (and prompt injection)

Tool results contain **board-derived text you do not control**: net names, component
references (`R1`, `J3`), layer names, and DRC descriptions. If you open a board authored
by someone else, that text can carry **indirect prompt-injection** payloads — e.g. a net
named `"ignore previous instructions and ..."` — which flow back into the calling model's
context.

What this server does and does not do about it:

- It **never interprets board content as commands.** Board text is parsed as data and
  returned verbatim; the server takes no action based on it.
- Its MCP `instructions` tell the client that tool outputs are untrusted data, not
  instructions.
- It **cannot** sanitize semantic injection away — defending the model against
  manipulation is the **client/model's** responsibility, not something any MCP server
  can guarantee. Treat all tool output as untrusted input to your agent.

## Input handling

- Subprocess arguments are passed via `std::process::Command` (argument vector, **no
  shell**), so there is no shell-injection vector.
- Client-supplied board paths are validated as existing files before reaching
  `kicad-cli`, which closes argument-injection (a flag-like `--foo` path) and yields a
  clean error.

## Known limitation

- **File access is not confined to a project root.** A path argument may reference any
  board file your user can read (information disclosure is limited — tools return
  counts/findings, not file contents). Confining access to a configured root (e.g. a
  `KICAD_MCP_ROOT`) is a planned optional hardening; until then, only connect this
  server to clients you trust with your filesystem.

## Code & dependency hardening

- `unsafe` code is **forbidden** workspace-wide (`unsafe_code = "forbid"`).
- Dependencies are checked in CI with `cargo deny`: RUSTSEC vulnerabilities and yanked
  releases are denied, licenses are allow-listed.

## Reporting a vulnerability

Please report security issues **privately** — do not open a public issue. Contact the
maintainer ([@brechanbech](https://codeberg.org/brechanbech) on Codeberg) and allow a
reasonable window to address it before any public disclosure.
