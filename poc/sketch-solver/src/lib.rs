//! PoC K-1 — own 2D sketch constraint solver (see `docs/spec/F11-solver-design.md`).
//!
//! Core logic is pure computation: no file system, threads or OS calls, so the crate builds for
//! `wasm32-unknown-unknown`. Transcendental functions come from `libm` for cross-platform determinism.
