# PoC results

## K-1 — own 2D sketch constraint solver in Rust · PASSED (2026-10-12)

Crate: `poc/sketch-solver/` (library, ~3.4 k lines in `src/`, plus ~0.9 k lines of tests and benchmarks).
Design: `docs/spec/F11-solver-design.md`. Kickoff: `poc/KICKOFF.md`. Branches `poc/k1-s1-scaffold` … `poc/k1-s8-results`,
each merged into `main` with `--no-ff`.

### Exit criteria (F11 §5)

| Criterion | Result | Evidence |
|---|---|---|
| SK-01…SK-19 + constraint matrix (F60–F67) as data-driven tests, all pass | **Pass** for every row in K-1 scope (table below); 17 fixture files, 64 cases, each run twice | `tests/conformance.rs`, `tests/fixtures/*.json` |
| H/V, coincident, point-on-curve, distance (aligned/H/V/point-line), parallel, perpendicular, collinear, equal, radius/diameter, concentric, tangent, midpoint, symmetric, angle, fix | **All implemented**, plus tangent circle–circle and equal radius; analytic Jacobians checked against finite differences for every kind | `src/equations.rs` tests |
| ≤ 50 ms for 500 entities / 600 constraints | **Pass**: 4–25 ms worst case on Windows, 1–7 ms on Linux (table below) | `benches/solve.rs` |
| DOF matches hand-computed values on 20 sample sketches | **Pass**: 25 samples, plus per-entity DOF on 9 of them | `tests/fixtures/dof_samples.json` |
| Results identical on Windows + Linux | **Pass**: bit-identical documents for all 64 cases (golden digest) | `tests/golden/digest.txt`, test `cross_platform_digest` |
| Builds for `wasm32-unknown-unknown` (web readiness) | **Pass**: core has no fs, thread or OS calls | `cargo build --target wasm32-unknown-unknown` |

### What was built

- **Model** (`model.rs`): stable u64 IDs from one never-reused counter. The document has points, lines,
  circles and arcs (arcs are centre + start + end + radius, with 2 internal equations). Constraints and dimensions
  share one record with a `driving` flag; `Fix` is also a constraint record, and the solver removes the fixed
  variables from the unknowns. Every entity carries `Provenance`; every command emits one `SketchOp` (H.3.18).
- **Equations** (`equations.rs`): 10 residual primitives with analytic gradients. Dimensionless rows (parallel,
  perpendicular, angle, direction) are scaled by the sketch size, so a single tolerance in mm applies to every row.
  Orientation signs (side of a tangent, sign of an H/V distance, angle sense) are taken from the geometry when the
  constraint is added and then stored. This is the branch selection.
- **Linear algebra** (`linalg.rs`): a sparse incremental LDLᵀ of `J W⁻¹ Jᵀ`. Rows are pivoted in a fixed
  order (internal equations, then constraints by ID, then soft goals). A row with a vanishing pivot is dependent:
  it is dropped and its *fundamental circuit* (the minimal dependent set) is recorded. That single factorization
  gives the rank (and so the DOF), the redundant or conflicting constraint plus its minimal set, the least-norm
  Gauss–Newton step, and the LM step (via the push-through identity). Per-entity DOF comes from the Schur
  complement of selector rows. nalgebra is used only in tests, as an SVD rank oracle.
- **Solver** (`solver.rs`), per cluster (connected component):
  1. Gauss–Newton on hard equations plus soft goals.
  2. Hard equations only.
  3. Newton with a tight pivot tolerance, for singular roots.
  4. LM least squares. If LM reaches a stationary point with a non-zero residual, the result is a conflict.
  - Minimal change (F61–F67) is expressed as soft goals appended after the hard rows (keep point, offset, length,
    direction, midpoint, radius, drag target). A goal that depends on the hard rows is dropped automatically. The
    movement table of F.3 lives in `EditContext::for_constraint`.
  - **Conflict search**: deletion filter, giving an irreducible infeasible subset (clusters with ≤ 64 constraints);
    otherwise the fundamental circuit.
  - **Branch guard**: if a corner orientation flips, the solve is retried by continuation on the dimension values.
  - **Degeneracy guard**: a zero-length line or zero radius is reported as `SKETCH.DIM.INVALID_VALUE`; a folded
    corner gives a warning (`SKETCH.GEOMETRY.DEGENERATE`).
  - **Collapse handling**: if a solve shrinks a curve to zero, it is retried once with that line nudged by 1e-3 rad
    and its length kept. If it still collapses, the result is infeasible (conflict).
- **Commands** (`doc.rs`, `SketchDoc`): each command validates, solves, then either commits exactly one undo step or
  is rejected with H.5 codes and leaves the document and undo stack unchanged. Commands: point, line, circle, arc,
  2-pt rectangle (adds its 2H + 2V, F36), constraint, measured dimension (F51), edit dimension, driving/driven
  (F38), delete (F95), drag (F231, S-16), rotate/scale (S-08), undo/redo. Undo/redo use snapshots, so redo is
  bit-identical by construction.

### Conformance coverage (artifact M.1)

| Row | Fixture | Result | Notes |
|---|---|---|---|
| SK-01 | sk01_rectangle | pass | DOF 4 → 2 → 1 → 0; per-entity DOF 0; area 800 (shoelace; profile detection is K-3) |
| SK-02 | sk02_dimension_toggle | pass | 20 toggles 40↔73.3: **1 distinct geometry, bit level** |
| SK-03 | sk03_redundant_dimension | pass | `REDUNDANT` + `make_driven`, no change; driven dimension reads 40, follows W=55; switching it to driving → `REDUNDANT` |
| SK-04 | sk04_invalid_values | pass | 0, −10, zero distance, negative radius, r=0 circle → `DIM.INVALID_VALUE`; document and undo depth unchanged |
| SK-05 | sk05_conflict | pass | Reference values (base 30, sides 25): 25→10 gives apex (23.75, 7.8062) = F105; then 30/10/10 → `SOLVE.CONFLICT` {base, s1, s2}; geometry and values kept |
| SK-06 | sk06_degenerate | pass | 30/15/15 → apex (15, 0) ± 1e-6 + `GEOMETRY.DEGENERATE`; recovers to (15, 20) |
| SK-07 | sk07_branch | pass | 10× 20↔28: apex y>0, exactly 2 positions (within 1e-9) |
| SK-08 | sk08_constraint_matrix | pass | 20 cases; H, V, parallel, perpendicular, equal, coincident, point-on-line, midpoint, concentric, equal circles reproduce the reference coordinates (`vmharness/results/constraint_matrix.json`, 4 decimals); collinear-from-perpendicular reproduces F109 exactly ((40,0)-(40,20) → (20,0)-(40,0)) |
| SK-09 | sk09_duplicates | pass | `DUPLICATE` in either argument order; also redundant H via parallel (set {par, h, new}) and conflicting V (set {par, h, new}) |
| SK-10/11/12 | — | out of scope | profile detection = PoC K-3 |
| SK-13 | sk13_undo_redo | pass | 5× undo restores each snapshot exactly; 5× redo bit-identical |
| SK-14 | — | out of scope | GUI inference thresholds (UI) |
| SK-15 | — | out of scope | projected geometry (needs the kernel / K-2) |
| SK-16 | sk16_reproducible | pass | under-constrained W 60↔40 ×10; the runner replays every case and requires a bit-identical result |
| SK-17 | sk17_fix_conflict | pass | H on a fixed oblique line → `CONSTRAINT.CONFLICT` {fix, H}; a distance on a fixed line likewise |
| SK-18 | sk18_symmetry | pass | true mirror of both endpoints (N.2a-2); points about an oblique axis |
| SK-19 | sk19_rotate_scale | pass | rotate 30° → `CONSTRAINT.REMOVED` listing the 4 H/V, dims kept, undo restores; 90° swaps H↔V; scale ×2 scales W/H to 80/40 |
| S-07 | s07_angle | pass | 45→30→120→45° rigid rotation about the shared point, length kept; 0/180/−10/200° → `DIM.INVALID_VALUE` |
| S-16 | s16_drag | pass | free end moves, merged origin stays (F231); drag along remaining freedom; fully constrained corner → `MOVE.CONSTRAINED`, no change |

### Performance (`cargo bench`, criterion, release)

Two sketches, each 500 curves and about 600 constraints in one connected cluster (worst case for decomposition):
- **ladder**: 451 lines + 49 circles, 603 constraints, almost linear.
- **chain**: 300 segments with aligned lengths and angles between neighbours, + 200 circles, 601 constraints,
  fully nonlinear. Editing one angle in the middle swings 150 segments; it converges in 4 iterations.

| Case | Windows i9-14900HX (Balanced plan) | WSL2 Ubuntu 22.04, same machine |
|---|---|---|
| ladder: edit one dimension, full analysis incl. per-entity DOF | 4.8 – 16.7 ms | 4.4 ms |
| ladder: edit one dimension, no per-entity DOF | 1.2 – 3.5 ms | 1.05 ms |
| ladder: initial solve from perturbed coordinates | 1.3 – 3.7 ms | 1.17 ms |
| ladder (under-constrained, 150 DOF): drag | 2.6 – 7.1 ms | 2.25 ms |
| chain: edit one angle, full analysis | 8.6 – 25.3 ms | 7.3 ms |
| chain: edit one angle, no per-entity DOF | 2.8 – 6.0 ms | 1.8 ms |

The Windows ranges come from a quick run and a full run of the same binary. The hybrid P/E cores on the Balanced
plan explain the spread. Per-entity DOF is the most expensive part (one sparse solve per entity). The UI can
request it lazily (`SolveOptions::per_entity_dof`).

### Determinism

- One thread per cluster. Every collection is a `BTreeMap` or sorted, so no hash order reaches the numerics.
  Transcendentals come from `libm`; Rust does not contract a*b+c into FMA.
- Same document + same edit sequence ⇒ bit-identical coordinates. Every fixture case runs twice in-process. The
  golden digest over all 64 final documents is identical on `x86_64-pc-windows-msvc` and
  `x86_64-unknown-linux-gnu` (rustc 1.99.0 on both).

### Decisions taken in the PoC (review)

1. **Tangent line–circle** translates the line purely (minimal change). The reference also rotates it by about 0.3°
   (F64 data). Same tangency, different by up to 0.1 mm.
2. **Collinear** of a free line keeps its length and moves it least. If an endpoint already lies on the first
   line's extension, the line rotates about that endpoint (F109). Otherwise the reference position differs
   (its rule is not observable).
3. **Collapse = infeasible**: a solve that shrinks a curve to zero size (e.g. Vertical on a horizontal line that is
   also Parallel to a horizontal line) is rejected as a conflict naming the minimal set. It also carries
   `SKETCH.DIM.INVALID_VALUE` with the collapsed curve (F11 §2.6).
4. **Rank loss at a degenerate configuration** (collinear triangle) is reported as `GEOMETRY.DEGENERATE` (warning),
   not as `REDUNDANT`.
5. **New diagnostic codes** proposed for H.5: `SKETCH.GEOMETRY.DEGENERATE` (warning) and `SKETCH.INPUT.INVALID`
   (error: unknown or wrong-kind reference, editing a driven dimension). They are appended to H.5 as [P].
6. **Dimensions** are constraint records with `driving: bool` rather than a separate `dims` collection.
7. **Sparse LDLᵀ instead of nalgebra dense QR/SVD**. The in-order pivot dropping gives deterministic "the newest
   constraint is the redundant one" semantics and stays fast at 600 rows. nalgebra remains a test oracle.

### Known gaps / follow-ups

- Expressions (EX-*), ellipse/spline/conic/slot/polygon/pattern constraints, offset and curvature (G2) are not
  in the PoC.
- Separating rigid-body DOF from internal DOF (F.6-3) for the learning mode is not done. It needs a 3-column
  rigid-motion selector, the same mechanism as per-entity DOF.
- The branch guard only watches corner orientation. Arc direction and tangent side are fixed by the stored signs;
  `TangentCurves` internal ties its sign to the radius order at compile time.
- Conflict search above 64 constraints per cluster falls back to the fundamental circuit (minimal for linear
  dependence, not guaranteed minimal for nonlinear infeasibility).
- No PlaneGCS oracle comparison yet (F11 names it as the comparison oracle). The reference coordinates from
  `constraint_matrix.json` / `solver_accept.json` stand in for it.
- An exactly symmetric tie (perpendicular collinear without an endpoint on the extension) is resolved by the
  deterministic 1e-3 rad nudge direction. That is reproducible, but the choice is arbitrary.
- Build output must stay outside the Synology-synced folder (`CARGO_TARGET_DIR`). Cargo could not persist
  `Cargo.toml` edits made by `cargo add` there ("Access is denied").
- WSL2 Ubuntu now has rustup with Rust 1.99.0 (user level, `~/.cargo`), used for the Linux determinism check.
