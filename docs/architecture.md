# Architecture

How the crates fit together, where a new feature goes, and which rules are enforced
by tests. The analysis that led to the current structure, with the refactoring plan,
is in [architecture-analysis.md](architecture-analysis.md).

## Crates and their direction

```
openecraft-i18n ──┐
openecraft-parts ─┼─► openecraft-jt / openecraft-step
                  └─► openecraft-core ─► openecraft-api ─► editor, cli
                           │                 ▲
openecraft-client ─► openecraft-ui ──────────┘ (editor, parts-manager)
parts-server (parts, i18n)        project-server (core, parts)
```

- Library crates live under `crates/`; executables are the root package (editor,
  `application/`), `cli/`, `parts-server/`, `project-server/`, `parts-manager/`.
- A crate depends only on crates to its left. No library depends on an executable.
  `openecraft-api` has no UI dependency and no `cfg(target_arch)` branch.
- Platform code (`wasm32` versus native) exists only in `crates/openecraft-ui/src/io.rs`
  and the `main.rs` files.
- Protocol types sit beside the logic that both sides share: parts server API in
  `openecraft_parts::api` / `portal`, project server API and merge rules in
  `openecraft_core::collab`.

## Layers inside `openecraft-core`

The crate is one compilation unit, so the compiler does not stop a module from
reaching upwards. `application/conventions/tests.rs` (`core_modules_respect_the_layer_rules`)
does: every module listed in `CORE_RULES` may refer only to the modules named there. The
table starts with the modules whose dependencies are already clean and grows as modules
are layered; a module that is not listed is not checked yet.

Target layers, bottom up (a module may use its own layer and the layers below):

1. `geom`, `xml`: no dependencies.
2. `symbols`, `symlib`: symbol definitions.
3. `model` (with its data submodules): the project file format. Data only, serde,
   no evaluation.
4. `netlist`, `index`, `structure`, `frame`, `wirenum`, `history`, `translation`,
   `options`: evaluation of the model that everything else builds on.
5. Feature logic: `panel`, `electrical`, `fluid`, `plc`, `plc_hw`, `field`,
   `routing3d`, `strips`, `plugs`, `potentials`, `harness`, `marking`, …
6. `analysis`, checks and reports: derived data shared by front ends, and the
   registry entries that produce issues and tables.
7. Output: `render`, `svg`, `pdf`, `dxf`, `dwg`, `export3d`, `aml`, `integration`.
8. Editing and exchange: `ops`, `macros`, `collab`, `archive`, `projects`, `verify`.

## Where a new feature goes

1. Data that is stored in the project: a type in the model layer, a field of
   `Project` (with `#[serde(skip_serializing_if)]` for empty values), a conversion
   step in `archive::convert` if an existing field changes meaning.
2. Logic: a module in the feature layer with its tests in `<module>/tests.rs`.
3. Checks, reports, exports, commands: registry entries (`openecraft_api::Registry`,
   see `docs/plugins.md`), so that the editor, the command line and the MCP server
   offer them at once. Front ends never special-case one of them.
4. UI: a module under `application/app/` that draws and sends actions; rules that
   change the project go to core (`ops` or the feature module) so the command line can
   use them.
5. Texts: English in the code, through `tr` / `tr_fmt!` / `tr_const!`, with catalog
   entries in all four languages (see AGENTS.md).

## Decisions

- **Formatting (2026-10-24).** The sources are not in rustfmt's shape: `cargo fmt
  --check` with `max_width = 160` reports 2 884 differing hunks (5 188 at the default
  width). A one-time reformat would conflict with every branch in flight, so there is no
  `rustfmt.toml` and `cargo fmt` is not run. Match the surrounding code by hand. Revisit
  when the branches are quiet; the reformat is then one commit of its own.
- **Long functions.** `clippy.toml` sets `too-many-lines-threshold = 150` and the editor
  and core enable `clippy::too_many_lines` outside tests. Existing offenders carry an
  explicit `#[allow(clippy::too_many_lines)]`; remove the allow when you split one.
- **Undo** stays snapshot based (`history.rs`): mutate the project freely, the history
  commits by comparison. Heavy payloads (part models, embedded documents) are shared
  with `Arc` so that comparison and cloning stay cheap.
