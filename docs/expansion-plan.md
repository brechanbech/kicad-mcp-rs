# Expansion plan: project setup, schematic editing, PCB inspection & edits

Forward plan beyond v0.1.0, covering three capability areas: **project setup**,
**schematic editing**, and **PCB inspection & edits**. It is grounded in what the three
backends can actually do (not just what we'd like), and it introduces one new
foundational piece — a native Rust `.kicad_sch` editor — that the rest depends on.

## 1. The constraint: what each backend can *edit*

Reading is easy everywhere; editing is not. From inspecting `kiutils-rs` 0.2,
`kicad-ipc-rs` 0.5 (KiCad 10 API), and `kicad-cli`:

| Capability | `files` (kiutils) | `cli` (kicad-cli) | `ipc` (live KiCad) |
|---|---|---|---|
| PCB read | rich, headless | DRC, render | rich, live |
| PCB **structural edit** | metadata only | — | **full** (transaction: create/update/delete items, layers, zones) |
| Schematic read | metadata / sexpr | ERC, netlist, BOM | — (can only *list* open schematics) |
| Schematic **edit** | metadata only | — | **none** (IPC is PCB-only) |
| Project config | lib-tables, design-rules CRUD | — | net classes, text variables |
| Project from scratch | — | — | — |

Three truths drive everything below:

1. **Live PCB editing is fully supported** via the IPC transaction model — the cleanest
   path to real board edits, but it needs a running KiCad GUI (not headless).
2. **Headless structural editing requires a sexpr editor we build** — `kiutils-rs`
   exposes only metadata/property/library-table edits; its board/schematic ASTs are
   read-only.
3. **Schematic editing needs a native Rust `.kicad_sch` editor that does not yet exist**
   in the ecosystem. This is the central new investment (§2).

### How the reference project does schematic editing

`oaslananka/kicad-mcp` (package `kicad-mcp-pro`, Python) edits schematics by manipulating
the `.kicad_sch` **file** through a dedicated structural library (`kicad-sch-api`), then
issuing an **IPC reload** so a running KiCad picks up the change. The IPC API itself is
PCB-only there too. Notably, even with that library most schematic operations are tagged
"wrapper_needed" rather than "native" — so structural schematic editing is real but
non-trivial work even with a head start.

**We studied this approach; we are not deriving from it** (see §6).

## 2. The new foundation: a native Rust `.kicad_sch` editor

The enabling piece for schematic editing. A `.kicad_sch` is an S-expression text file,
so structural editing is tractable — the gap is purely that no Rust library does it.

**Methodology — independent reimplementation, validated empirically.** We implement
against KiCad's documented S-expression file format and validate by **lossless
round-trip of real KiCad 10 files** (parse → write back byte-identical), then by editing
and reloading them in KiCad 10. This is the same "validate against real KiCad output"
discipline used for the DRC model and the `kiutils` fixtures. We do **not** translate
`kicad-sch-api`'s source — a file format is not copyrightable, and reimplementation keeps
us free of derivative-license entanglement.

**Leverage, don't reinvent.** Where practical, build on `kiutils`'s lossless S-expression
CST (`kiutils_sexpr`) rather than writing a new parser; the new work is the
**structural-edit layer** on top.

**Phasing.**
- **S0 — Round-trip.** Parse `.kicad_sch` and write it back byte-identical (lossless),
  with unknown-node preservation. Fixtures from real KiCad 10 projects (clean,
  multi-sheet, hierarchical). This is the safety net for every edit.
- **S1 — Read model.** Typed view: symbols (lib_id, reference, value, position,
  properties), wires, labels (local/global/hierarchical), buses, junctions, no-connects,
  sheets/hierarchy, nets.
- **S2 — Structural edits.** Add / move / delete symbols (with library-symbol resolution),
  wires, labels, buses, no-connect markers; set/update properties; place from a symbol
  library.
- **S3 — Higher-level ops.** Annotation (reference assignment), basic wire routing between
  pins (the A\* idea — reimplemented), bulk property edits, ERC-aware checks.

**Packaging — its own crate (§3).**

## 3. Crate & repo structure (decision)

The `.kicad_sch` editor is a **general-purpose library** — useful far beyond this MCP
server, and a genuine gap in the Rust KiCad ecosystem (which already has standalone
`kiutils-rs` and `kicad-ipc-rs`). It also carries **KiCad-version coupling** and will want
its **own release cadence** and test corpus. That argues strongly for a **standalone
crate in its own repository**, published to crates.io, consumed by `kicad-mcp-rs` as a
dependency — mirroring the ecosystem pattern.

Proposed name: **`kicad-sch-rs`** (consistent with `kiutils-rs` / `kicad-ipc-rs`).

Two viable development paths:

- **Own repo from day one** *(recommended).* Clean history, clean attribution boundary,
  independent versioning from the start. `kicad-mcp-rs` depends on it by `path` during
  development, switching to a version dependency once published. Slightly more setup
  (new repo + CI), but the boundaries are honest immediately.
- **Incubate-then-extract.** Develop it as a `crates/` member here for fast iteration,
  then split to its own repo once the API is round-trip-stable. Faster to start, but the
  history entangles and extraction is fiddly.

Given it is unambiguously a reusable library and you already lean toward its own repo, I
recommend **own repo from day one**. `kicad-mcp-rs` gains a `kicad-sch` access path
(thin wrapper, like `cli`/`files`/`ipc`) once the crate is usable.

## 4. The three capability areas

### A. PCB inspection & edits *(most feasible; lands first)*

- **Inspect (headless, `files`):** extend beyond summary/geometry → enumerate footprints,
  nets, layers, stackup, tracks/vias as MCP tools.
- **Inspect (live, `ipc`):** read the open board — items, selection, nets, pad netlist,
  bounding boxes, hit-test.
- **Edit (live, `ipc` only):** transaction-based create/update/delete items, set
  layers/stackup/title-block, refill zones, net classes. Headline new edit capability;
  requires a running KiCad. Behind the safety gate (§5).

### B. Project configuration *("setup", realistically scoped)*

- Library tables (`kiutils` CRUD: add/remove/rename fp & sym libs), design rules
  (`.kicad_dru` CRUD), net classes + text variables (`ipc`), title block.
- Optional/later: scaffold a new project from minimal templates (write
  `.kicad_pro` + empty board/schematic). *From-scratch board/schematic content generation
  is out of scope until the editors are mature.*

### C. Schematic *(inspection now; editing once `kicad-sch-rs` lands)*

- **Now (no new library):** ERC via `kicad-cli sch erc` → gate outcomes (mirrors DRC),
  netlist / BOM export, structure read.
- **After `kicad-sch-rs` (§2):** structural editing tools — place/move/delete symbols,
  wires, labels, buses, no-connects; annotation; property edits — then **IPC reload** to
  refresh a running KiCad (file-edit + reload, as the reference does).

## 5. Safety model for editing

Editing changes the threat model: read tools leak at most some metadata; **write tools let
a prompt-injected client modify the user's design or files.** Non-negotiables for any edit
tool:

- **Off by default** — gated behind an explicit feature/config, not enabled by a bare run.
- **Dry-run / preview** — describe the change before applying it.
- **Never auto-commit** — lean on the IPC transaction model (atomic, user-undoable in the
  UI) and, for file edits, write via the lossless round-trip with a backup.
- **Clear labelling** — tool descriptions state "mutates your design," and outputs remain
  untrusted data (per `SECURITY.md`).

## 6. Attribution & licensing

- The `.kicad_sch` editor is **independent work** against KiCad's public file format, not
  a port of `kicad-sch-api` or `kicad-mcp-pro`. Per [`ATTRIBUTION.md`](../ATTRIBUTION.md),
  studying their *approach* owes nothing; record them as **studied references, not derived
  from**. If we ever port a specific algorithm (e.g. their A\* wire router), that single
  module gets the header + a note — the boundary stays auditable.
- Any new dependency passes `cargo deny` (licenses + advisories) before adoption.

## 7. Suggested sequencing

1. **Schematic + PCB inspection/validation** — ERC→gates, netlist/BOM, extended PCB read
   tools. Cheap, safe, headless; builds straight on v0.1.0.
2. **Project configuration** — lib-tables / `.kicad_dru` / net-classes / text-variables.
3. **`kicad-sch-rs` (own repo)** — S0→S3 in parallel as the foundational library; the
   biggest single effort.
4. **Live PCB edits (`ipc`)** + **schematic edits (via `kicad-sch-rs`)** — behind the
   safety gate (§5). Highest power, highest risk.
