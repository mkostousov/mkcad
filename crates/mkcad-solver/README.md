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

Cross-platform determinism: `tests/golden/digest.txt` holds an FNV-1a digest of every fixture's final document
(bit level). The same digest must come out on Windows (MSVC) and Linux, e.g. from WSL2:

```text
wsl -d Ubuntu-22.04 -- bash -lc 'cd /mnt/d/.../poc/sketch-solver && CARGO_TARGET_DIR=$HOME/.cargo-target/mk_cad cargo test'
UPDATE_GOLDEN=1 cargo test --test conformance cross_platform_digest   # only after an intended numeric change
```

The repository lives in a synced folder (Synology Drive); keep build output outside it, e.g.
`$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cargo-target\mk_cad"`.
