# AGENTS.md

Guidelines for AI coding agents (and humans) working on OpenECraft – an EPLAN-style
electrical CAE editor in Rust (egui/eframe) that runs natively on Windows, Linux,
macOS and in the browser (WebAssembly).

## Build, test, lint

Run these before considering a change done. All of them must pass without warnings.

```bash
cargo test
cargo clippy --all-targets
cargo clippy --target wasm32-unknown-unknown
```

- Desktop: `cargo run --release`
- Web: `trunk serve` (dev server on http://127.0.0.1:8080), `trunk build --release`
- The web target needs `rustup target add wasm32-unknown-unknown` and `cargo install trunk --locked`.

## Unit tests – mandatory layout

**Unit tests never live inline in a module file.** Each module keeps its tests in a
separate file next to it:

```
src/geom.rs          ← module code, ends with:
                         #[cfg(test)]
                         mod tests;
src/geom/tests.rs    ← the tests, starting with `use super::*;`
```

Rules:

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
  empty boxes (the browser hides the problem). `src/glyphs/tests.rs` checks this; if
  it fails, pick a character the fonts contain instead of disabling the check. Keep
  the egui feature `monochrome_emoji_fonts` enabled.
- **eframe features:** default features are disabled in `Cargo.toml`; when changing
  the feature list keep `clipboard` (desktop copy/paste), `persistence`,
  `default_fonts` and `glow`.
- **Demo project:** `src/sample.rs` is checked by `sample/tests.rs` (no check errors,
  no warnings). Keep it consistent when changing symbols, parts or checks.
- **Rendering:** draw through the `render::Draw` trait so the canvas and SVG / print
  export stay identical.
- **Undo:** edits are captured by snapshot comparison (`history.rs`); just mutate the
  `Project`, no explicit undo bookkeeping is needed.
- **Platform code:** keep `#[cfg(target_arch = "wasm32")]` branches confined to
  `io.rs` and `main.rs`.
- **Style:** match the surrounding code; keep comments as sparse as the existing ones.

## Commits

- Commit only when asked. Use an imperative summary line and a short body listing
  the changes.
- Never commit IDE folders (`.idea/` is ignored) or build output (`target/`, `dist/`).
