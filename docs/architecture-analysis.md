# Architecture analysis and refactoring plan

Date: 2026-10-24. Baseline: `master` at a77311a (130 commits, 2026-10-19 to 2026-10-24),
`cargo check --workspace --all-targets` clean in 2 min 04 s.

Method: read every crate root, the dependency sections of all manifests, the central
modules (`model`, `reports`, `render`, `history`, `symbols`, `symlib`, `collab`, `ops`,
`properties`), the editor (`app.rs` and its 29 submodules, `editor.rs`, `panel_ui.rs`),
the command line (`tools.rs`, `session.rs`, `mcp.rs`, several tool modules), the plugin
API, both servers, both clients and the parts manager; built an intra-crate dependency
map of `openecraft-core` from its `use crate::…` lines; scanned function lengths and
git change frequency.

## Status (2026-10-24)

All seven phases were carried out on the branch `worktree-refactoring`, one commit per
phase; the three gate commands pass after each.

| Phase | Commit | Result |
| --- | --- | --- |
| 0, 1 | ddc6d88 | Layer test `CORE_RULES`, `too_many_lines` limit, `docs/architecture.md`; `Shared<T>` for part models and embedded documents, report tables built on demand (`Analysis::report`), help text fixed. Not done: the whole-tree `cargo fmt` (decision in architecture.md); 1d is superseded by phase 5. |
| 2a-2d | 45e7ef0, bcb427a, 2d8d78d, 28e6d99 | Registry kinds, parameters and tables in `openecraft_core::{ext, params, table}`; the built-in checks, exporters and reports are registry entries published by built-in plugins; reports are addressed by name (`ReportId`, file format 2 with a conversion step). |
| 3a-3d | f46dd17, 229688a | Project data in `model/<feature>.rs`, logic in the feature modules; `Analysis` and `Issue` in modules of their own; the whole module graph pinned in `CORE_RULES` (8 layers, upward edges listed). No crate split: a clean `cargo check` of core takes 16 s. |
| 4 | e5f053f | `Session` (document state, undo, analysis refresh, tested) apart from `App`; `app.rs` is the composition root, menus, files, tool bar, navigator, property panel and windows are modules of their own; the editor imports core modules explicitly. Not done: 4e (`panel_ui` split) and the full dialog-state conversion of 4c. |
| 5 | dad56fe | `Project::symbol` / `symbol_table()` with a per-project `SymbolCache` instead of `symlib::register`: no leaking registry, no per-frame registration, `SymbolRef` for built-in and user definitions. |
| 6 | a3f0559 | 151 project tools of the command line are commands of 23 built-in area plugins (`crates/openecraft-api/src/builtin/<area>.rs`, tested through `builtin::testing`); the command line keeps the tools that need the session or the machine, the editor's Plugins menu has one submenu per plugin. Kept in the command line on purpose: `add_page`, `set_report_block` and `set_report_scheme`, which validate report names against the registry a command cannot see. |
| 7 | see git log | `Issue` carries a template with its arguments (`issue!`, `Issue::message()` renders it in the current language) or a plain text; the built-in checks use templates, plugin and generated texts stay plain, the `check` tool reports `key` and `args`. Not converted: problem lists that return strings (`plugs`, `structure`, `singleline`, `fluid`, `electrofluid`, `process`) and messages assembled before the issue is built. |

Of the open questions below, 1 and 3 are answered by the decisions in
`docs/architecture.md`, 2 was done (format 2 with conversion), 4 stays open: the Plugins
menu groups the commands by plugin, a command palette would be the next step.

## 1. Summary

The architecture is sound in its large shape and the project is unusually well guarded
for its age: the crate graph points in one direction, platform code is confined, the
four-language rule and the test layout are enforced by tests, and the `Draw` trait keeps
screen, SVG, PDF and DXF identical. Nothing here needs a rewrite.

The structural debt is concentrated in four places, all of them consequences of adding
~100 k lines of features in six days onto a design made for a much smaller editor:

1. **`openecraft-core` is one 52 k-line crate whose module graph is a tangle around
   `model`, `reports` and `render`.** `Project` embeds a type from 30 feature modules,
   and every feature module imports `model` and `reports`. Nothing in core can be built,
   tested or understood apart from the rest; every feature touches the same three files.
2. **Two extension systems exist side by side.** `openecraft-api::Registry` is the
   documented way to add checks, reports and exporters, but the 41 built-in reports,
   ~25 built-in checks and 17 export formats are hard-wired into `Analysis::build`, a
   closed `ReportKind` enum (enumerated in seven places), the editor's File menu and the
   CLI's `export` match. The two lists are already out of sync: the CLI help text names
   31 of the 41 reports.
3. **The editor `App` is a god object**: ~60 fields, 2 400 lines in `app.rs`, 29
   `impl App` blocks spread over 29 files, a 425-line menu function, and business rules
   (page order, imports) executed inline in menu handlers. `main.rs` re-exports all 60
   core modules as `crate::…`, which hides the boundary between UI and model.
4. **Per-frame cost grows with project size.** Undo compares the whole `Project`
   structurally every frame and clones it on every commit; parts carry 3D meshes and
   pages carry base64 documents inline, so both operations walk megabytes. Every committed
   change regenerates all 41 reports, whether or not anyone looks at them.

A fifth, smaller problem has correctness implications: user symbols live in a
process-wide, leaking, mutable registry (`symlib::register`), consulted by
`symbols::get` from 89 call sites.

The plan below is ordered so that each phase ships on its own, keeps the three gate
commands green, and leaves the code better even if the next phase never happens. Phases
1, 4 and 5 are independent of each other and of 2 and 3; phase 3 (layering core) should
precede any crate split; phase 6 is optional and incremental.

| Phase | Goal | Size | Depends on |
| --- | --- | --- | --- |
| 0 | Guardrails: architecture tests, formatting config, this document | S | none |
| 1 | Cheap cost fixes: shared blobs, lazy reports, stale help text | M | none |
| 2 | One extension system: built-in checks, reports, exporters go through the registry | L | 0 |
| 3 | Layer `openecraft-core`: data vs logic, enforced by a test; crate split only if measured | L | 2 |
| 4 | Decompose the editor: explicit imports, a `Session` type, self-contained dialog modules | L | none (4c after 2) |
| 5 | Symbol table instead of the global registry | M | none |
| 6 | CLI tools become built-in commands available everywhere | XL, incremental | 2 |
| 7 | Structured check issues (kind + parameters instead of translated strings) | M | 2 |

## 2. What works and must be kept

- **Crate graph with one direction**: `i18n` → `parts` → `jt`/`step` → `core` → `api` →
  front ends; `client` and `ui` beside them; servers depend only on `parts` / `core`.
  No crate depends on an application crate. `openecraft-api` is free of UI and `cfg`
  branches as AGENTS.md demands.
- **Platform code confined**: the only `cfg(target_arch = "wasm32")` lines are in
  `crates/openecraft-ui/src/io.rs` and the two `main.rs` files. `FileIo<K>` with the
  `FileKind` trait is a clean seam.
- **Conventions enforced by tests**: `openecraft/src/conventions/tests.rs` (test layout,
  untranslated widget texts), `openecraft/src/glyphs/tests.rs` (font coverage), the i18n
  catalog tests (four languages, placeholders, every `tr` literal present),
  `sample/tests.rs` (demo project has no check errors). The 543 core tests and the 99
  CLI tests (black-box through JSON) are a real safety net for refactoring.
- **`render::Draw`** as the single rendering abstraction with four back ends (egui, SVG,
  PDF, DXF): keep it exactly as is.
- **Snapshot undo** (`history.rs`): the idea (mutate freely, commit by comparison) is
  right for this editor. Only its cost needs work (phase 1).
- **The plugin API design** (`Registry`, typed `params`, external JSON protocol,
  `Plugins::add` name checks, `Text` as borrowed catalog key or owned string): good
  shape, under-used (phase 2).
- **Servers** are thin: `parts-server` with a `Backend` trait (SQLite / JSON) and a
  `Store` holding the rules; `project-server` with `Store` + `Sessions` and the merge
  rules in `core::collab` where clients can test them.
- **Parts manager** keeps its editing rules in `session.rs` without UI, as the guidelines
  ask. This is the pattern the editor should follow (phase 4).
- **CI**: three operating systems plus the web build, clippy with `-D warnings`.

## 3. Metrics

| Crate | Lines | Files | Tests | Transitive deps |
| --- | ---: | ---: | ---: | ---: |
| `openecraft` (editor, `openecraft/`) | 18 960 | 48 | 30 | 347 |
| `openecraft-core` | 52 625 | 183 | 543 | 54 |
| `openecraft-cli` | 9 902 | 63 | 99 | 82 |
| `openecraft-parts` | 5 887 | 31 | 70 | 14 |
| `openecraft-parts-server` | 2 720 | 13 | 31 | |
| `openecraft-api` | 2 336 | 15 | 26 | 55 |
| `openecraft-ui` | 2 319 | 13 | 18 | 231 |
| `openecraft-parts-manager` | 1 186 | 4 | 12 | |
| `openecraft-project-server` | 839 | 7 | 6 | |
| `openecraft-jt` | 640 | 5 | 6 | |
| `openecraft-client` | 472 | 5 | 7 | |
| `openecraft-i18n` | 401 | 4 | 7 | |
| `openecraft-step` | 318 | 5 | 7 | |

Other numbers that matter for the plan:

| Measure | Value |
| --- | ---: |
| Fields of `Project` | 44 |
| Modules `model.rs` imports a type from | 30 |
| Modules `reports.rs` imports | 32 |
| Modules `render.rs` imports | 20 |
| `ReportKind` variants | 41 |
| Places that enumerate `ReportKind::ALL` outside core | 7 |
| `ReportKind` mentions outside core | 51 |
| Sources of check issues wired into `Analysis::build` | 25 |
| Distinct return shapes of those check functions | 6 |
| `Analysis::build` call sites in CLI tools | 26 |
| `symbols::get(` call sites (36 files) | 89 |
| `impl App` blocks | 29 |
| Functions over 100 lines / over 200 lines (workspace) | 73 / 14 |
| `pub const ALL` closed-enum tables in core | 51 |
| `tr(` / `tr_fmt!(` calls in core | 1 886 |

Longest functions:

| Lines | Function |
| ---: | --- |
| 841 | `symbols::build_library` (`crates/openecraft-core/src/symbols.rs:498`) |
| 473 | `integration::records` (`crates/openecraft-core/src/integration.rs:645`) |
| 449 | `PanelUi::properties` (`openecraft/src/panel_ui.rs:715`) |
| 426 | `PanelUi::view_3d` (`openecraft/src/panel_ui/view3d.rs:52`) |
| 425 | `App::menu_bar` (`openecraft/src/app.rs:564`) |
| 371 | `App::windows` (`openecraft/src/app.rs:1811`) |
| 349 | `PanelUi::front_view` (`openecraft/src/panel_ui.rs:1736`) |
| 256 | `RevisionsUi::ui` (`openecraft/src/app/revisions_ui.rs:46`) |
| 246 | `App::symbol_properties` (`openecraft/src/app.rs:1564`) |
| 238 | `routing3d::analyse` (`crates/openecraft-core/src/routing3d.rs:634`) |

Most frequently changed files in git (the three hubs of finding F1 lead):
`app.rs` 43 (31 as `src/app.rs` + 12), `reports.rs` 36, `lib.rs` 28, `model.rs` 33,
`cli/src/tools.rs` 24, `panel.rs` 17, `cli/src/tools/output.rs` 17.

## 4. Findings

### F1. `openecraft-core` is a cyclic hub, not a layered model

`Project` (`crates/openecraft-core/src/model.rs:25`) holds 44 fields, 30 of which are
types owned by feature modules (`crate::potentials::PotentialDef`,
`crate::field::Field`, `crate::integration::Integration`, `crate::review::Review`, …).
Each of those modules in turn imports `model`, and most import `reports` for `Issue`,
`Severity` and `Table`. The dependency map therefore has `model → 30 modules`,
`reports → 32 modules`, `render → 20 modules`, and almost every module → `model`,
`reports`. Examples of direct cycles: `model ↔ potentials`, `reports ↔ safety`,
`reports ↔ panel`, `netlist ↔ fluid`, `panel ↔ manufacturing`.

Consequences:

- Core is one 52 k-line compilation unit; it is the longest link in every build
  (editor, CLI, both servers, parts manager web build all wait for it).
- No module has a defined public surface; `pub fn` count in core is 1 306.
- Every feature edits `model.rs` (field), `reports.rs` (report and check wiring) and
  `lib.rs` (module), which is exactly what the git statistics show.
- Nothing stops a check from calling into the renderer or an export from reaching into
  a UI-only concept, because there is no layer to violate.

### F2. Two extension systems; the built-in one is closed and partly stale

`Analysis::build` (`crates/openecraft-core/src/reports.rs:107`) is a hand-written
orchestrator: a 41-arm `match` producing every report, followed by 25 `issues.extend`
lines whose sources return six different shapes (`Vec<Issue>`,
`Vec<(String, Option<_>)>`, `Vec<(Severity, String, Option<_>)>`,
`Vec<(String, bool, Option<_>)>`, `Vec<String>`, `Vec<Problem>`) adapted inline.

`ReportKind` (`model.rs:244`) is a closed enum with `ALL`, `title()`, and it is
enumerated outside core in seven places (`app.rs` four times, `forms_ui.rs`,
`report_blocks_ui.rs`, `cli/src/tools/common.rs`). It is also part of the file format
through `PageKind::Report(ReportKind)`, so plugin reports can never become report pages.

Export formats are listed twice by hand: the editor's File menu (`app.rs:564` onwards)
and the CLI's `export` match (`cli/src/tools/output.rs:138`). Import kinds are the
25-variant `Purpose` enum in `openecraft/src/io.rs` dispatched by a 172-line if-chain in
`App::poll_loaded`.

Meanwhile `openecraft-api::Registry` offers exactly the abstractions needed (`Check`,
`Report`, `Exporter`, `Command`), the front ends already iterate it
(`plugins_ui.rs`, `cli/src/tools/output.rs:76`), and AGENTS.md says every new check,
report or export must go through it. Only four built-in plugins do. The result is two
`Table` types (`reports::Table` with widths and `&'static str` headers,
`api::Table` with `Text` headers) and two code paths in every front end.

Concrete symptom: the `report` tool's help text (`cli/src/tools/output.rs:24`) names 31
reports; `ReportKind::ALL` has 41. The ten newest (`routing_list`, `bundle_list`,
`short_circuit`, `selectivity`, `load_calculation`, `power_supply_budget`,
`safety_functions`, `verification`, `field_routing`, `bonding`) are missing from what
MCP clients are told.

Cost symptom: every committed change in the editor rebuilds all 41 tables
(`app.rs:2249`), and a CLI `export` or `report` call rebuilds the analysis from scratch
(26 call sites in the tools), including 3D routing and field planning, even for an SVG
of one page.

### F3. The editor `App` is a god object with hidden boundaries

- `App` (`openecraft/src/app.rs:66`) has ~60 fields mixing document state (project,
  history, analysis, index, file name, remote session), UI state of 25 dialogs, and
  caches. 29 submodules each add an `impl App` block, so any dialog can read or write
  any field; the struct is the only interface.
- `menu_bar` is 425 lines, `windows` 371, `symbol_properties` 246, `properties` 238,
  `navigator` 206, `poll_loaded` 172. `PanelUi::properties` is 449 lines.
- Business rules run inline in menu handlers: "Move page up" does
  `self.prj.pages.swap(...)` in the menu (`app.rs`, Page menu), while the CLI has to
  provide its own page operations. 42 UI functions take `&mut Project` and mutate it
  directly; the undo mechanism makes that safe, but it means the editor and the CLI
  share less logic than they should.
- `openecraft/src/main.rs:20` re-exports all 60 core modules, so editor code is written as
  `use crate::model::*`, `crate::reports::Analysis`, `crate::svg::page_svg`. A reader
  cannot tell UI from model, and moving code between the crates leaves no trace. (The
  history shows why: the editor used to be `src/` containing everything.)
- `EngineeringUi` (`openecraft/src/app/engineering.rs`) bundles four unrelated windows
  (macros, terminal strips, PLC overview, marking) in one struct.

### F4. Per-frame work scales with the whole project

`History::track` (`crates/openecraft-core/src/history.rs:23`) runs once per frame
(`app.rs:2376`) and evaluates `*prj != self.committed`, a derived `PartialEq` over the
entire `Project`; on a commit it clones the project and keeps up to 200 clones.

Baselines and revision snapshots are already behind `Arc` with a pointer-equality fast
path (`compare.rs:17`, `revision.rs:19`), which is the right technique. It is not
applied to the two remaining heavy payloads:

- `Part.model_3d: Option<Mesh3d>` and `model_2d` (`crates/openecraft-parts/src/part.rs:375`)
  inside `prj.parts`: a project with a few hundred parts with meshes is compared
  triangle by triangle every frame and copied on every undo step.
- `Document.data: String` (base64 file content, `document.rs:15`) in `Page.document`
  and in every `ImageItem` (`model.rs:855`).

`Index::build` additionally runs every frame while a pointer button is down or a text
field has focus (`app.rs:2259`), and `symlib::register` runs every frame (`app.rs:2246`).

### F5. Global mutable symbol registry

`symlib::register` (`crates/openecraft-core/src/symlib.rs:118`) leaks every changed
user symbol into a process-wide `RwLock<HashMap>` (`Box::leak`), and
`symbols::get` (`symbols.rs:1346`) falls back to it. 89 call sites in 36 files resolve
symbols by key with no context. Consequences:

- Symbols removed from a project, or from a previously opened project, stay resolvable
  ("ghost" definitions); the comment in `register` accepts this.
- Hosts that handle several projects in one process see cross-talk by key: the
  project server (`Store::open` loads every project), the project database scan
  (`projects::Entry::read` with `verify` runs `Analysis::build` on many files), the MCP
  session across `open_project` calls, and `cargo test` threads.
- The editor must call `register` every frame to keep the registry in step.
- Memory grows with every edit in the symbol editor.

### F6. Dependency hygiene

- `openecraft-core` links `acadrust` (a full DWG reader/writer) and `openecraft-jt` only
  for `dwg.rs` and `export3d::jt`. The project server, which needs model + collab only,
  compiles both.
- `openecraft-ui` depends on `openecraft-step` (the `monstertruck` CAD kernel) and
  `openecraft-jt` only for `part_editor::parse_model_3d`; this puts the kernel into the
  parts manager's web build.
- `openecraft-parts` (described as "free of UI and platform dependencies") also carries
  the HTTP API types and portal rules (`api.rs`, `portal.rs`, ~1 100 lines). Acceptable
  because it adds no dependencies, but the name no longer says what the crate is.
- Three client layers for the parts server: `openecraft-client::Endpoint` (requests),
  `openecraft-ui::client::ServerClient` (asynchronous, `Reply` enum),
  `cli::parts_server::Server` (blocking, 28 one-line wrappers). The blocking wrapper
  could be generic in `openecraft-client`.
- No `rustfmt.toml`, and the code is not in rustfmt's default shape (lines of 160+
  columns throughout), so `cargo fmt` cannot be used without a decision.

### F7. Editor and CLI duplicate the same feature lists

The CLI's `Tool` (`fn(&mut Session, &Args)`) and the API's `Command`
(`Fn(&mut Project, &Args)`) are two shapes of the same thing. 27 tool modules
(~10 k lines) are the product's real command surface, and the editor cannot see them:
it has no command palette and its menus call core directly. Both front ends keep their
own list of exports, reports and imports (F2). AGENTS.md already states the target
("front ends never special-case a built-in plugin; they only show what `Plugins`
offers"); the code predates the rule.

### F8. Smaller items

- `PageKind::Report(ReportKind)` ties the file format to the closed enum (F2).
- `reports::Table.headers: Vec<&'static str>` relies on `tr` returning static strings;
  fine, but it is why two `Table` types exist.
- Check issues are translated strings built at generation time, so the analysis must be
  rebuilt on a language change and MCP clients cannot filter by kind.
- Done: the root manifest is a virtual workspace, the editor is the crate
  `openecraft/` like every other member (it was the root package, sources in
  `application/`).
- Only one `TODO` in the whole tree and eight `#[allow]`s (all
  `too_many_arguments`): the code is clean at the small scale; the debt is structural.

## 5. Refactoring plan

Principles for every phase:

- One phase, often one sub-step, is one pull request against `master`. The three gates
  from AGENTS.md (`cargo test --workspace`, both clippy runs) stay green at every
  commit; no gate is weakened.
- Behaviour does not change unless the step says so (phase 1c and 2d change user-visible
  things and say what).
- New seams get a test that pins them (a layering test, a registry round-trip test), so
  the next feature cannot undo the refactoring by accident.
- Measure before the expensive moves: record `cargo build` wall time and the per-frame
  cost on the demo project before phases 1 and 3, and after.

### Phase 0: Guardrails (S)

Goal: make the intended structure visible and checkable before moving code.

1. Add `docs/architecture.md` (short, maintained): crate graph with the direction rule,
   the layers inside core introduced in phase 3, where a new feature goes (data type,
   logic, registry entry, UI), and a link to this analysis. Reference it from AGENTS.md.
2. Add an architecture test next to `openecraft/src/conventions/tests.rs` that parses
   `use crate::…` lines of core and asserts a dependency rule table. Start with the rules
   that already hold (`geom` imports nothing; `symbols` imports only `geom`, `symlib`;
   `history` imports only `model`; `i18n` has no dependencies). Phase 3 extends the
   table as layers are carved out.
3. Decide on formatting: add `rustfmt.toml` with `max_width = 160` (matching the
   existing style), run `cargo fmt` once in a commit of its own, and add
   `cargo fmt --check` to CI. If the one-time diff is unwanted, skip this item; do not
   leave the question open.
4. Add `clippy.toml` with `too-many-lines-threshold = 150` and enable
   `clippy::too_many_lines` as a warning in `openecraft/src/main.rs` and `core/lib.rs`
   with `#![warn]` plus explicit `#[allow]` on the existing offenders, so new long
   functions are caught while old ones are reduced over time.

Acceptance: CI unchanged except for the new checks; no production code moved.

### Phase 1: Cheap cost fixes (M)

Goal: stop the per-frame and per-change cost from scaling with project size, without
touching the undo design. Each item is a separate PR.

1a. **Shared heavy blobs.** Wrap `Mesh3d` and `Model2d` in `Part` and `Document.data`
    in `Arc` with the pointer-equality fast path already used by `compare::Stored` and
    `revision::Snapshot` (serde `rc` is enabled in core; enable it in
    `openecraft-parts`). Files: `crates/openecraft-parts/src/part.rs`,
    `crates/openecraft-core/src/document.rs`, call sites that construct or replace
    models and documents (parts UI, model import, document pages). Test: a project with
    a large mesh compares and clones in constant time (assert with a counter or by
    `Arc::ptr_eq` after `clone`).

1b. **Lazy, memoised reports.** `Analysis` keeps the shared derived data (netlist, open
    netlist, index, connections, wiring, routing, field plan, duct fill, changes) and
    computes tables on demand: `Analysis::report(&self, kind) -> &Table` with a
    `OnceCell` per kind (or `RefCell<HashMap>`), invalidated with the analysis. The
    `HarnessData` special case becomes a normal lazy table. The editor only asks for the
    table it shows or prints; `report_blocks::page_table` asks per page kind. Checks that
    need a table (`check_harnesses`) call `report`. Expect the common edit to cost one
    netlist + index build instead of 41 tables. Also keep `Index` inside `Analysis` so
    `App` stops rebuilding it separately.

1c. **Stale CLI help.** Replace the hand-written report list in
    `cli/src/tools/output.rs:24` by a `list_reports` tool (built-in kinds via
    `ReportKind::ALL` with `snake(k)`, plus plugin reports) and point the `report` help
    text at it. Add a CLI test that every `ReportKind` is accepted by `report --kind`.
    User-visible: new tool, shorter help text.

1d. **Per-frame registration.** Call `symlib::register` only when `prj.symbols` changed
    (compare a cheap hash or the history revision) instead of every frame. Interim
    measure until phase 5 removes the registry.

1e. **`poll_loaded` dispatch.** Turn the if-chain into a `match loaded.purpose` with
    one handler function per purpose group; no behaviour change. Prepares phase 4.

Acceptance: demo project edit loop measurably cheaper (record numbers in the PR);
all tests unchanged.

### Phase 2: One extension system (L)

Goal: the built-in checks, reports and exporters are entries of the same registry the
plugins use, so front ends have one code path and the lists cannot diverge. This is the
phase AGENTS.md already describes; it is split so each step is reviewable.

2a. **Registry types move down to core.** Create `openecraft_core::ext` holding the
    data-only types now in `openecraft-api::registry` and `params::Text` (`Check`,
    `Report`, `Exporter`, `Table`, `Text`, the function type aliases). `openecraft-api`
    re-exports them (public API unchanged) and keeps `Plugin`, `Plugins`, `Command`,
    `params` and `external`. Merge the two `Table` types: `ext::Table` gets `widths`
    (optional, default equal) and `Text` headers; `reports::Table` becomes an alias and
    is removed once nothing names it. Test: existing api and report tests pass unchanged.

2b. **Checks through the registry.** Give every check source in `Analysis::build` the
    signature `fn(&Project, &Analysis) -> Vec<Issue>` (adapting the six shapes at their
    source, not in `reports.rs`), and register them in a built-in table
    `core::checks::BUILTIN: &[Check]` grouped by area (devices, wires, potentials,
    panel, fluid, electrical, …) with a snake_case name and a title. `Analysis::build`
    runs the table; `Plugins::check` appends plugin checks as today. User-visible later:
    checks can be listed and switched off by name (not in this step).

2c. **Exporters through the registry.** Each of the 17 formats in
    `cli/src/tools/output.rs:138` and the File menu becomes an `Exporter` entry in
    `core::exporters::BUILTIN` (name, title, extension, binary flag, options such as
    page and language as `params`). The CLI `export` tool and the editor's File menu
    iterate the merged list (built-in + plugins). `Exporter` needs two additions the
    API lacks today: binary output (`Vec<u8>`, for DWG and JT) and typed options; add
    them to `Registry`, consume them in both front ends and in `external.rs`, document
    them in `docs/plugins.md`, keep `API_VERSION` (additive).

2d. **Reports through the registry and a string key.** Register the 41 tables as
    `Report` entries (`core::reports::BUILTIN`), replace `ReportKind` with a `ReportId`
    string newtype across the model (`PageKind::Report(ReportId)`, `ReportBlock.kind`,
    `ReportScheme.kind`, `Forms`), bump `model::FORMAT` to 2 with a conversion step in
    `archive::convert` (variant name → snake_case id) and a regression test with a
    format-1 file. Front ends enumerate `Plugins::reports()` only; the seven
    `ReportKind::ALL` loops disappear. User-visible: plugin reports can be report pages
    and report blocks.

Acceptance: `grep ReportKind` finds nothing outside the conversion code; the editor's
Reports window, the CLI `report`/`export` tools and the MCP tool list are generated
from one list; `docs/plugins.md` updated.

### Phase 3: Layer `openecraft-core` (L)

Goal: a module graph with a direction, enforced by the phase-0 test, so that a change
to one feature stays in one place and a later crate split is mechanical rather than
heroic.

3a. **Data below logic.** For every feature module whose type is a field of `Project`,
    move the serde data types (and only those) into a `model/` submodule
    (`model/potentials.rs`, `model/field.rs`, …) and leave the logic where it is.
    `Project` then imports only `model::*`; the feature module imports `model` and its
    own data. Do this one feature per PR (30 small PRs or a few grouped ones). The
    layering test gets a rule per completed module: `model/*` imports only `geom`,
    `symbols` and `model/*`.

3b. **Base types out of `reports`.** `Issue`, `Severity` and `Table` move to
    `core::ext` (done in 2a); `Analysis` moves to `core::analysis` with the derived data
    only; the 41 table functions move next to their features
    (`panel::reports`, `electrical::reports`, `fluid::reports`, …) and register from
    there. `reports.rs` disappears. Rule: `analysis` imports `netlist`, `index`,
    `panel::wiring`, `routing3d`, `field`; nothing imports `analysis` except reports,
    checks, render, exports and front ends.

3c. **Define the layers and lock them in the test**, bottom up:
    `geom` → `symbols` → `model` → `netlist`, `index`, `structure`, `wirenum` →
    feature logic (`panel`, `electrical`, `fluid`, `plc_hw`, `field`, `routing3d`, …) →
    `analysis`, `checks`, `reports` → `render`, `svg`, `pdf`, `dxf`, `dwg`,
    `export3d`, `aml`, `integration` → `ops`, `macros`, `collab`, `archive`. A module may
    import from its own layer or below. Cycles that remain (e.g. `netlist ↔ fluid`) are
    resolved by moving the one function that points upward.

3d. **Optional crate split, by measurement.** Only if `cargo build` time or the
    web-build size justify it after 3a–3c: cut along the layer lines into
    `openecraft-model` (geom, symbols, model, archive conversion), `openecraft-analysis`
    (netlist, index, analysis, checks, reports), `openecraft-exchange` (svg, pdf, dxf,
    dwg, export3d, aml, integration, with `acadrust` and `jt` behind cargo features),
    and keep `openecraft-core` as a facade that re-exports them so no front end changes.
    This removes `acadrust` and `jt` from the project server and gives the parts manager
    a smaller web build.

Acceptance: layering test green with the full table; `model.rs` imports no feature
module; build time recorded before and after.

### Phase 4: Decompose the editor (L)

Goal: `App` becomes a thin composition root; dialogs are self-contained; business rules
live in core where the CLI can use them.

4a. **Explicit imports.** Delete the `use openecraft_core::{…}` re-export block in
    `openecraft/src/main.rs` and rewrite `crate::model::…` etc. as `openecraft_core::…`
    (mechanical, one PR, no logic change). The conventions test can forbid
    re-exporting core modules from `main.rs` so it does not come back.

4b. **A `Session` type for the document state.** Extract from `App` into
    `openecraft/src/session.rs`: `prj`, `history`, `analysis`, `index`, `saved_rev`,
    `file_name`, `remote`, `page`, with a small API (`edit(|prj| …)`, `goto`,
    `replace_project`, `dirty`, `analysis()`), unit-testable without egui (the pattern
    `parts-manager/src/session.rs` already follows). `App` keeps UI state only. Dialog
    modules receive `&mut Session` instead of `&mut App`.

4c. **Self-contained dialog modules.** Turn each `impl App` extension block into a
    struct with `fn window(&mut self, ctx, session: &mut Session) -> Vec<Action>`
    (the `Action` enum several modules already use locally becomes one shared enum:
    `Goto`, `Notify`, `Export`, `OpenFile(Purpose)`, …). `App::windows` becomes a loop.
    Split `EngineeringUi` into its four windows. Do one module per PR, biggest first
    (`revisions_ui`, `structure_ui`, `engineering`, `projects_ui`).

4d. **Menus and rules.** Split `menu_bar` into one function per menu. Move inline rules
    (`pages.swap`, delete page clamps, rename device, add page position) to `core::ops`
    and use them from both the editor and the CLI tools (`cli/src/tools/pages.rs`).
    After phase 2c the File menu's export section is a loop over exporters.

4e. **Panel UI.** Apply 4c to `PanelUi::properties` (449 lines) and `front_view`
    (349): one function per property section (climate, checks, line-up, routing, wire
    are already separate; devices, profiles, enclosure follow).

Acceptance: `app.rs` under ~800 lines; no function in `openecraft/` over 150 lines
except the egui canvas loops; `Session` has tests for open / save / dirty / goto.

### Phase 5: Symbol table instead of a global registry (M)

Goal: symbol resolution is a value, not process state.

1. Introduce `symbols::Table` (built-in definitions plus the project's user symbols as
   `Arc<SymbolDef>`), built by `Project::symbol_table()` (cached behind the history
   revision or a hash of `prj.symbols`).
2. Change `symbols::get(key)` into `Table::get(&self, key)` and thread `&Table` through
   the 89 call sites. Most are inside functions that already receive `&Project` or
   `&Analysis`; put the table into `Analysis` and `Index` so the editor, render, netlist
   and reports read it from there, and give `ops`/`macros` a `&Table` parameter.
3. Delete `symlib::register`, `lookup`, the `RwLock` and the `Box::leak`; delete the
   per-frame call in `App::ui` and `Project::normalize`.
4. Test: two projects with the same user key in one process resolve to their own
   definitions; a removed symbol is no longer found.

Acceptance: `grep -r "static REG"` empty; no `Box::leak` in core.

### Phase 6: CLI tools as built-in commands (XL, incremental, optional)

Goal: the 27 CLI tool modules become `Command`s of built-in plugins so the editor
(Plugins menu, later a command palette) and the MCP server offer the same operations
from one definition, as AGENTS.md intends.

- Keep in the CLI only what needs the session or the machine: `open_project`,
  `new_project`, `save_project`, `undo`, `redo`, `output_reports`, the `parts_*` tools
  (parts server), `plugins`.
- Move the project tools area by area (`pages`, `drawing`, `devices`, `wiring`, …) into
  `openecraft-api::builtin::<area>` as commands with the same names, params and help
  texts; their tests move with them (run through `Plugins` as the API tests do). The CLI
  `Entry` enum then has far fewer `Tool`s. Reserved-name checks already exist.
- `Command` needs a `&Analysis` argument for tools that read derived data (several
  build their own today, see F2); add it as an optional second argument or give
  commands a `Context { prj, analysis }`.

Each area is one PR; stop at any point with a consistent system.

### Phase 7: Structured check issues (M, optional)

Replace `Issue { message: String }` by `Issue { kind: IssueKind, args: Vec<String>, … }`
rendered through `tr` at display time. Benefits: no analysis rebuild on language change,
stable ids for MCP clients and for a future "suppress this check" feature, and the
external plugin protocol can keep sending plain messages (`IssueKind::Plugin(String)`).
Depends on 2b (checks in a table).

### Suggested order and parallelism

1. Phase 0, then 1a–1e (one week of small PRs; immediate payoff).
2. Phase 2a–2d (the biggest design change; do it before phase 3 because 3b depends on
   the registry types living in core).
3. Phases 4a, 4b and 5 can run in parallel with phase 2 since they touch different
   files (`openecraft/` vs `core`); 4c/4d after 2c.
4. Phase 3a–3c after 2; 3d only with numbers.
5. Phases 6 and 7 when the product wants them (command palette, check suppression).

## 6. What not to do

- No rewrite, no second UI framework, no async runtime in the editor: egui's immediate
  mode with snapshot undo fits this product.
- No crate split before the layering test passes; splitting a cyclic graph produces
  `pub(crate)` leaks and a facade crate that re-exports everything, which is worse than
  one crate.
- No weakening of the convention tests, the i18n catalog test or the glyph test to make
  a refactoring step pass.
- Do not move translated texts out of core "to make it pure": generating reports and
  check messages in the user's language inside core is a deliberate decision in
  AGENTS.md; phase 7 changes how, not where.
- Do not introduce a trait-object `Project` or getters/setters for its fields; the
  snapshot undo depends on plain mutation, and the pub fields are fine once the data
  layer is separated (phase 3a).

## 7. Open questions for the maintainer

1. Formatting (phase 0.3): accept one large `cargo fmt` commit with `max_width = 160`,
   or leave formatting manual?
2. File format bump (phase 2d): acceptable to raise `FORMAT` to 2 now, or should
   `ReportKind` keep its enum name in files for a while (both readable, new id written)?
3. Crate split (phase 3d): is build time a pain today? If yes, record the numbers so
   the split can be judged against them.
4. Phase 6 scope: should the editor get a command palette (which would make the
   migration of CLI tools user-visible), or is MCP/CLI parity the only goal?
