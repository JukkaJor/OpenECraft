# Patched copy of proc-macro-error2 2.0.1

Used through `[patch.crates-io]` in the root `Cargo.toml`. It is a dependency of
`step-p21-derive` (STEP import, `crates/openecraft-step`).

The only change against the released crate is in `src/lib.rs`:

```rust
pub extern crate proc_macro;   // was: extern crate proc_macro;
```

2.0.1 re-exports the private `extern crate proc_macro` (`pub use proc_macro;` in
`__export`), which Rust is phasing out (rust-lang/rust#127909) and reports as a
future incompatibility on every build. Remove this copy and the patch entry once a
release of `proc-macro-error2` contains the fix.
