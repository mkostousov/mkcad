# PoC kickoff — own 2D sketch solver in Rust (K-1)

Read first (in this order): `docs/PLAN-next.md`, `docs/spec/F11-solver-design.md`, `docs/spec/F-sketch-solver.md`
(F.3 constraint table, F.5–F.9), `docs/spec/M-conformance-suite.md` (SK-*, EX-*), `docs/spec/N-open-questions.md` §N.2a
(decisions), `docs/spec/S-silent-outcomes.md`. Evidence: `docs/STUDY.md` (Fnn).

## Decisions that bind the PoC
- Rust (stable), workspace at `mk_cad/` root later; this PoC lives in `poc/sketch-solver/` (library crate + tests).
- Own solver: unknown vector of point coords / radii; analytic Jacobian; rank analysis for DOF + redundancy; LM /
  least-norm Gauss–Newton with minimal-change weighting; conflict subset search; degeneracy guard; deterministic order.
- Symmetry = true mirror of both endpoints (N.2a-2). Break: no constraint between pieces; Trim: Collinear (N.2a-3).
- Expression argument separator "," (N.2a-6) — expressions are not part of this PoC unless time permits.
- Diagnostics use the codes of `docs/spec/H-data-contracts.md` §H.5 (machine codes, not strings).

## Work plan
1. `cargo new --lib poc/sketch-solver`; deps: `nalgebra` (dense QR/SVD for small clusters) — sparse later; `serde`,
   `serde_json` for test fixtures; `approx` for tests.
2. Data model: `PointId`, `CurveId`, `ConstraintId` (stable u64/ULID), `Sketch { points, curves, constraints, dims }`.
3. Residuals + Jacobians per constraint (F11 §1), each with unit tests against finite differences.
4. Solver core: cluster decomposition → rank/DOF → weighted least-norm GN/LM → convergence/conflict/degeneracy.
5. Fixtures: JSON cases for SK-01…SK-19 and the constraint matrix (F60–F67) under `tests/fixtures/`; one test per
   fixture asserting coordinates (tol 1e-9 mm), DOF, status codes.
6. Bench: 500 entities / 600 constraints ≤ 50 ms (criterion).
7. Log results in `poc/RESULTS.md`; update `docs/PLAN-next.md` Phase 3 status.

## Cross-platform setup (decided 2026-10-12)
- rustup + `rust-toolchain.toml` pinning stable + rustfmt + clippy; native builds per OS (no cross-compiling with C++ deps).
- Targets: Windows `x86_64-pc-windows-msvc` (VS 2022 Build Tools; NOT the GNU target — OCCT/vcpkg are MSVC),
  macOS `aarch64-apple-darwin` (+x86_64 optional, Xcode CLT), Linux `x86_64-unknown-linux-gnu` (gcc/clang).
- Workspace: pure-Rust crates (solver, domain, persistence) + one `occt-sys`/`kernel` crate (cxx + cmake build) — only it needs C++.
- OCCT from vcpkg on all OSes, dynamically linked (LGPL). CI later: GitHub Actions matrix windows/macos/ubuntu.
- Determinism: use the `libm` crate for transcendental functions in the solver; stable ordering; WSL2 Ubuntu for local Linux checks.

## GPU configuration (decided 2026-10-12)
- Rendering: `wgpu` (Vulkan/Metal/DX12); GPU ID-buffer picking, clipping-plane sections, instancing, MSAA, high-DPI.
- GPU never computes authoritative geometry: solver + OCCT on CPU in f64 (cross-platform determinism).
- AI: ONNX Runtime (`ort`) with CUDA/DirectML/CoreML, or `candle`; cloud LLM optional; AI acts only via the command API (undoable).
- Development of rendering/AI happens on the HOST (RTX 4070 laptop) and WSL2 — never in the VirtualBox VM (no real GPU).
- Host extras when needed: Vulkan SDK (renderer debugging), CUDA Toolkit only if CUDA is preferred over DirectML.

## Sketch observability readiness (decided 2026-10-12, UI later)
- Every entity carries `Provenance { createdBy, groupId, modifiedBy }`; every sketch command returns one `SketchOp`
  (created/modified/deleted IDs) — see `docs/spec/H-data-contracts.md` §H.3.18. Solver returns per-entity DOF.

## Web readiness (decided 2026-10-12)
- The solver crate must build for `wasm32-unknown-unknown`: no std::fs / threads / OS calls in core logic; add a
  `cargo build --target wasm32-unknown-unknown` check to the PoC (rustup target add wasm32-unknown-unknown).
- Web edition later: solver + wgpu in browser, OCCT as a remote kernel service behind `IGeometryKernel`.

## Environment notes
- Host (verified 2026-10-12): rustup with stable 1.99.0 `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown`,
  clippy, rustfmt (`C:\Users\mikhail\.cargo\bin` — open a new terminal/chat if `cargo` is not on PATH);
  VS Community 2026 and VS Build Tools 2022 with C++ tools; CMake; Git. Native + wasm32 smoke builds pass.
  `mk_cad` is not yet a git repository (ask the user before `git init`).
- Usage: check plan usage periodically; at ~95 % pause and schedule a resume per the user's rule.
