# AGENTS.md

Guidelines for AI coding agents (and humans) working on OpenECraft – an EPLAN-style
electrical CAE editor in Rust (egui/eframe) that runs natively on Windows, Linux,
macOS and in the browser (WebAssembly).

## Build, test, lint

Run these before considering a change done. All of them must pass without warnings.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo clippy -p openecraft -p openecraft-parts-manager --target wasm32-unknown-unknown
```

- Desktop: `cargo run --release`
- Parts server: `cargo run -p openecraft-server -- --token <secret>` (http://127.0.0.1:8090)
- Parts manager: `cargo run -p openecraft-parts-manager`; web: `cd parts-manager && trunk serve` (http://127.0.0.1:8082)
- Web: `trunk serve` (dev server on http://127.0.0.1:8080), `trunk build --release`
- The web target needs `rustup target add wasm32-unknown-unknown` and `cargo install trunk --locked`.

## Workspace layout

| Crate | Path | Purpose |
| --- | --- | --- |
| `openecraft` | `.` (`src/`) | Editor (desktop + web) – the root package, built by trunk |
| `openecraft-parts` | `crates/openecraft-parts` | Shared library: part model, function templates, CSV/JSON, catalog, parts server API types |
| `openecraft-ui` | `crates/openecraft-ui` | Shared egui library: part tree / editor / previews, parts server client, file dialogs |
| `openecraft-server` | `server/` | Standalone parts database server (axum, JSON file storage) |
| `openecraft-parts-manager` | `parts-manager/` | Standalone parts server maintenance app (desktop + web, own `index.html` / `Trunk.toml`) |

- Code needed by more than one executable belongs in a library crate under `crates/`,
  never duplicated.
  - `openecraft-parts` stays free of UI and platform dependencies (only `serde` /
    `serde_json`); pure logic such as 3D math (`vec3`) goes here.
  - `openecraft-ui` holds egui widgets and platform code shared by the editor and the
    parts manager. It must not depend on an application crate.
- API types and request paths of the parts server live in `openecraft_parts::api`; change
  server and client together.
- New crates are added to `[workspace] members` in the root `Cargo.toml`; the
  convention test reads that list, so its rules apply to every member.

## Unit tests – mandatory layout

**Unit tests never live inline in a module file.** Each module keeps its tests in a
separate file next to it:

```
src/geom.rs          ← module code, ends with:
                         #[cfg(test)]
                         mod tests;
src/geom/tests.rs    ← the tests, starting with `use super::*;`
```

Rules (for every crate of the workspace):

1. No `#[test]` functions and no `mod tests { … }` blocks in module files.
2. The module file declares its tests exactly as `#[cfg(test)]` followed by `mod tests;`
   (at the end of the file).
3. The test file is `src/<module>/tests.rs` and starts with `use super::*;` (it is a
   child module, so private items stay testable).
4. Test-only modules (e.g. `glyphs`, `conventions`) are declared in `main.rs` with
   `#[cfg(test)]` and follow the same layout: helpers in `src/<module>.rs`, tests in
   `src/<module>/tests.rs`.
5. When adding a new module with tests, create `src/<module>/tests.rs` right away – do
   not add an inline block "for now".
6. New behaviour gets a test. Bug fixes get a regression test.

These rules are enforced by `src/conventions/tests.rs` – `cargo test` fails on
violations. Do not weaken or delete that test to make a change pass.

## Other conventions

- **Glyphs:** every character used in a string literal must exist in the bundled
  egui fonts. Native builds have no font fallback, so missing characters render as
  empty boxes (the browser hides the problem). `src/glyphs/tests.rs` checks this for
  every UI crate (editor, `openecraft-parts`, `openecraft-ui`, parts manager); if
  it fails, pick a character the fonts contain instead of disabling the check. Keep
  the egui feature `monochrome_emoji_fonts` enabled.
- **eframe features:** default features are disabled in both app manifests; when changing
  the feature list keep `clipboard` (desktop copy/paste), `persistence`,
  `default_fonts` and `glow`.
- **Demo project:** `src/sample.rs` is checked by `sample/tests.rs` (no check errors,
  no warnings). Keep it consistent when changing symbols, parts or checks.
- **Rendering:** draw through the `render::Draw` trait so the canvas and SVG / print
  export stay identical.
- **Undo:** edits are captured by snapshot comparison (`history.rs`); just mutate the
  `Project`, no explicit undo bookkeeping is needed.
- **Platform code:** keep `#[cfg(target_arch = "wasm32")]` branches confined to
  `crates/openecraft-ui/src/io.rs` and the `main.rs` files.
- **Parts manager:** keep its editing rules (drafts, renames, unsaved changes) in
  `parts-manager/src/session.rs` (no UI code, unit tested); `app.rs` only draws.
- **Style:** match the surrounding code; keep comments as sparse as the existing ones.

## Commits

- Commit only when asked. Use an imperative summary line and a short body listing
  the changes.
- Never commit IDE folders (`.idea/` is ignored) or build output (`target/`, `dist/`).
