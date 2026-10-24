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
does: `CORE_RULES` lists every module with the modules it may refer to (`crate::…`
paths outside comments). The table pins the current graph: a new edge fails the test,
removing an edge means shortening a list. Entries that point upwards are the known
exceptions, to be removed over time.

Layers, bottom up (a module may use its own layer and the layers below):

1. Foundations without dependencies: `geom`, `xml`, `ttf` (TrueType reader and
   subsetter for PDF fonts), `params` (typed parameters and
   `Text`), `table` (report tables), `issue` (`Issue`, `Severity`; a built-in check's
   message is an English template with its arguments, `issue!(severity, target, "…",
   args)`, translated by `Issue::message()` where it is shown, so that the analysis
   survives a language change and MCP clients get a stable `key`; plugin and generated
   texts are `Issue::plain`).
2. `symbols`, `symlib`: symbol definitions. The built-in library is static
   (`symbols::builtin`); the user symbols of a project are resolved through
   `Project::symbol` / `Project::symbol_table()` (`symlib::Symbols`, a snapshot the
   editing functions take as a parameter), never through process-wide state.
3. `model` with its data submodules (`model/panel.rs`, `model/forms.rs`, …): the
   project file format. Data only, serde, no evaluation. The feature modules re-export
   their data types from here and keep the logic.
4. Evaluation of the model that everything else builds on: `index`, `netlist`,
   `structure`, `frame`, `wirenum`, `history`, `translation`, `options`,
   `potentials`, `functions`, `properties`, `props`, `numbering` (device and page
   renumbering), `text` (free texts: lines,
   wrapping, rotation, path function texts).
5. Feature logic and page editing: `ops`, `arrange` (align, distribute, multiple copy), `panel`, `electrical`, `fluid`, `plc`,
   `plc_hw`, `field`, `routing3d`, `strips`, `plugs`, `harness`, `marking`, `forms`,
   `report_blocks`, `revision`, `review`, `compare`, …
6. Derived data and the registry: `analysis` (`Analysis`: index, netlists,
   connections, panel wiring, routing, field plan, report tables on demand), `checks`
   (the built-in checks as `Check` entries), `reports` (the built-in tables as `Report`
   entries), `exporters` (the built-in exports as `Exporter` entries), `ext` (the
   registry kinds).
7. Output and exchange formats: `render`, `svg`, `pdf`, `dxf`, `dxf_import`, `dwg`,
   `export3d`, `aml`, `integration`.
8. Project files, sharing, verification: `archive`, `collab`, `projects`, `verify`,
   `sample`.

Known upward edges (listed in `CORE_RULES`, to be removed): `structure`, `macros`,
`page_templates` and `document` use `ops`; `field`, `document` and `review` draw with
`render` (and `field` with `svg`); `production` writes DXF; `panel` uses `analysis`
(routed wiring), `plc_hw` uses `ops`; `model` and `symlib` refer to each other
(`Project` carries the `symlib::SymbolCache` of its user symbols). `analysis` and `ext`
refer to each other (the registry kinds take an `&Analysis`).

## The editor

- `application/session.rs` holds the document state: the open project, its undo
  history, the analysis (rebuilt after committed changes and language changes, with the
  reports and checks of the registry), the device index, the saved revision and the
  file name. It has no UI code and is unit tested; `App` adds the windows.
- `application/app.rs` is the composition root: the `App` struct, start-up, the frame
  loop and the handful of helpers every window needs (`notify`, `goto`,
  `replace_project`, `add_page`). Menus (`app/menus.rs`), files (`app/files.rs`),
  tool bar and library (`app/toolbar.rs`), navigator, property panel and the main
  windows (`app/windows.rs`) are `impl App` blocks in modules of their own, one
  concern each; the dialogs under `app/` follow the same pattern with a state struct
  per dialog.
- Rules that change the project (page order, deleting pages, renaming devices) live
  in `openecraft_core::ops`, so the command line and the MCP server share them.
- The editor names core modules as `openecraft_core::…`; `main.rs` re-exports
  nothing (checked by `conventions/tests.rs`).

## Where a new feature goes

1. Data that is stored in the project: a type in the model layer, a field of
   `Project` (with `#[serde(skip_serializing_if)]` for empty values), a conversion
   step in `archive::convert` if an existing field changes meaning.
2. Logic: a module in the feature layer with its tests in `<module>/tests.rs`.
3. Checks, reports, exports, commands: registry entries (`openecraft_api::Registry`,
   see `docs/plugins.md`), so that the editor, the command line and the MCP server
   offer them at once. A built-in check goes into `openecraft_core::checks::builtin`,
   a built-in report into `reports::builtin` (one `ReportKind` with a name and title
   key), a built-in export into `exporters::builtin`; the built-in plugins of
   `openecraft-api` publish these lists. A command on the project is an entry of the
   `COMMANDS` table of its area in `crates/openecraft-api/src/builtin/<area>.rs`
   (`fn(&mut Project, &Args) -> Result<Value, String>`, tested through the
   `builtin::testing` session). Only tools that need the session or the machine
   (project files, undo, exports to files, the parts server) stay `Tool`s in
   `cli/src/tools/`. Front ends never special-case one of them: they enumerate
   `Plugins`.
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
- **No crate split (2026-10-24).** After the layering, `cargo check -p openecraft-core`
  from a clean build of the crate takes 16 s on the development machine, and the
  longest module files are `model.rs` (1 260 lines) and `reports.rs` (1 005 lines).
  That does not justify splitting the crate; the layer table gives the boundaries a
  split would follow if build time ever demands it (`model` + foundations, `analysis`
  with checks and reports, the output formats with `acadrust` and `jt` behind
  features).
- **Report names.** Since file format 2 pages, report blocks and forms name reports
  by their snake_case name (`ReportId`), so reports of plugins can be report pages.
  `ReportKind` stays the enum of the built-in kinds; files of format 1 are converted
  when opened.
