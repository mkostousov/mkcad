# sketch-solver — PoC K-1

Own deterministic 2D geometric constraint solver in Rust. Design: `docs/spec/F11-solver-design.md`;
kickoff and decisions: `poc/KICKOFF.md`; results: `poc/RESULTS.md`.

## Commands (from this folder)

```text
cargo test                                      # unit + fixture conformance tests
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown     # web-readiness check (core must stay OS-free)
cargo bench                                     # criterion: 500 entities / 600 constraints
```

The repository lives in a synced folder (Synology Drive); keep build output outside it, e.g.
`$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo-target\mk_cad"`.
