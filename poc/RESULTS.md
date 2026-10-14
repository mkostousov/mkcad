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

## K-2 setup — OCCT kernel spike (`poc/kernel-spike/`) · smoke tests PASSED (2026-10-13)

Scope: only the kernel bridge that K-2 (topological naming, REF-01…08) needs. No naming scheme yet.

### What was built

- `occt-sys`: C++ shim (`cpp/shim.cc`) + `cxx` bridge. Box, planar polygon extrude (prism), translate, constant-radius
  fillet by edge index, cut, fuse, volume, validity, face/edge area/length/centroid, STEP export and import.
  OCCT exceptions are caught in the shim and surface as Rust `Err`.
- `kernel`: safe `Solid` API and `Model` (linear operation history: name, params, volume, face/edge counts and the
  sub-shape mapping of each step).
- History: every operation returns `HistoryEntry`s taken from `BRepBuilderAPI_MakeShape` (`Modified`, `Generated`,
  `IsDeleted`) plus `Unchanged` (same `TShape` is in the result), over edges and faces, per operand.
- OCCT 8.0.1 from vcpkg (`x64-windows`, dynamic), MSVC 14.51. The vcpkg build took 26 min and was still running
  when this session started.

### Smoke tests (`kernel/tests/smoke.rs`, 7/7)

| Test | Checks |
| --- | --- |
| box | 20×20×20 → 8000 mm³ (tol 1e-6), 6 faces, 12 edges, valid |
| extrude | square 20 × h 20 → 8000 mm³; 4 profile edges each generate one distinct 20×20 side face |
| fillet | 1 vertical edge, r = 2 → 8000 − (1 − π/4)·r²·L = 7982.83 mm³; edge deleted, generates 1 face of area π/2·r·L; 4 faces modified |
| cut | through-hole 10×10 → 6000 mm³, 10 faces; top face modified into a 300 mm² face; 4 tool walls tracked |
| STEP | export, `MANIFOLD_SOLID_BREP` present, import → 8000 mm³ |
| model | box → fillet → cut recorded in order, volumes strictly decrease |
| errors | zero-size box, 2-point polygon, bad edge index, oversize fillet radius, missing file → `Err`, no crash |

### OCCT 8 findings (matter for the naming work)

1. **The fillet face is reachable only through `Generated(edge)` of a *deleted* edge.** A scan that skips deleted
   edges loses the fillet face. The first version of the shim had this bug.
2. **An edge generates a face**, so a history record needs a source kind and a destination kind (`dst_dim`).
3. `Generated(face)` of a prism returns the solid, not the top cap. The prism's top cap is not in the per-edge
   history (it comes from `LastShape()`); K-2 needs that explicitly.
4. Sub-shape indices are `TopExp::MapShapes` order and valid for one shape value only. They are not a stable name.
5. OCCT 8 API changes hit: `Standard_Failure` is a `std::exception` (`ExceptionType()`, no `DynamicType()`),
   `TopTools_ListIteratorOfListOfShape` is gone (range-for over `NCollection_List`), `TopTools_*Map*` headers are
   deprecated aliases.

### Setup notes

- Build: `CARGO_TARGET_DIR=C:\dev\cargo-target`, `OCCT_ROOT` defaults to `C:/dev/vcpkg/installed/x64-windows`.
- Run tests with `C:\dev\vcpkg\installed\x64-windows\bin` on `PATH` (dynamic OCCT DLLs).
- Linux/WSL build is not tried; `build.rs` would need `OCCT_ROOT` and lib names without `.lib` handled (it checks
  `lib<name>.so`).
- Not covered: shell/offset, variable-radius fillet, fillet failure modes on real parts, performance, determinism of
  the history ordering across platforms (K-4).

## K-2 — topological naming on OCCT history · REF-01…08 PASSED (2026-10-13)

Code: `poc/kernel-spike/kernel/src/naming.rs`, tests `kernel/tests/naming.rs` (10/10; the 7 kernel smoke tests still pass).
Scheme = G.4 steps 1–5 on extrude + box cut + fillet + shell + sketch-on-face.

### Scheme as built

1. **Face labels from authoritative inputs:** extrude → `f:side:<profile curve id>`, `f:cap:start`, `f:cap:end`; cut →
   `f:tool:<x-|x+|y-|…>` from the tool box definition; fillet/shell-generated faces → `f:gen:<source label(s)>`.
   OCCT indices are used only inside one rebuild.
2. **History propagation:** a result face inherits the label set of every input face that maps to it (modified /
   unchanged); a split face's pieces all keep the label.
3. **Descriptor:** a face label, or for an edge the sorted pair of the two adjacent face labels (derived, so edges
   need no history of their own), plus a signature (kind, centroid, normal, area/length).
4. **Resolution:** all faces/edges carrying the label → `exact` (1) or `expanded(n)`; none → signature within
   1e-6 → `signature` (1 hit) / `ambiguous` (>1) / `lost`.
5. **Recompute:** `Timeline::rebuild()` replays features from scratch; a feature with a lost/ambiguous reference
   reports `REF.LOST` / `REF.AMBIGUOUS`, health Error, and the previous body continues ("cached geometry").
   Descriptors are never overwritten, so removing the cause recovers automatically (F44).

### Replay of M.1 REF cases

| Case | Test | Result |
| --- | --- | --- |
| REF-01 unrelated upstream insert | hole inserted before the fillet | `exact`, volume = box − fillet − hole |
| REF-02 edge split | groove splits the top edge | `expanded(2)`, both 8 mm pieces filleted, valid |
| REF-03 edge removed → recovers | corner cut removes the vertical edge; then cut suppressed | `lost` + `REF.LOST`, body = pre-fillet; then `exact`, original fillet volume |
| REF-04 sketch on face, resized | extrude height 20 → 30 | plane follows to z = 30, `exact` |
| REF-05 face deleted under sketch | slab cut removes the top face | `lost`, `SKETCH.REF.LOST`, cached plane z = 20 |
| REF-06 split support | groove splits the top face | policy Single → `ambiguous` + `REF.AMBIGUOUS`; policy AllPieces (coplanar) → `expanded(2)`; never a silent pick |
| REF-07 shell face split | groove, then shell on the pre-groove top ref | `expanded(2)`, both faces removed, valid |
| REF-08 split edge re-merged | groove suppressed | `expanded(2)` → `exact` |
| extra: signature fallback | profile curve ids renamed, geometry same | `signature` |
| extra: determinism | two rebuilds | identical reports, volume, labels |

### Findings

1. **History was complete enough** for extrude, box cut, fillet and shell on these cases, once three OCCT quirks are
   handled in the adapter: the fillet face only appears as `Generated` of a *deleted* edge; the prism end cap only
   via `LastShape()`; `Generated` for an edge yields a *face*.
2. **A shell leaves rim faces.** Removing a face does not remove everything in its plane: a ring-shaped rim of wall
   thickness stays there (160 − 6×18 = 52 mm² per piece) and inherits the removed face's label. Whether a later
   reference to "the removed face" should resolve to the rim is a product decision.
3. **Edge labels derived from the two face labels** make REF-02/08 fall out without edge history, but two different
   edges between the same pair of face labels (a face split into two pieces that both touch the same neighbour)
   cannot be told apart: they always expand together. That is the spec's wanted behaviour for F42, but it also means
   one of two *deliberately* distinct edges cannot be addressed alone. The signature does not help (label match wins).
4. **A face with several labels** is supported in the data model but the edge pair uses the smallest label. A merge
   op (fuse of coplanar faces) would need the descriptor to carry all of them. Not exercised.

### Decisions to review

- `PiecePolicy` is per consuming feature (Single → `REF.AMBIGUOUS`, AllPieces needs coplanar pieces). The PoC does
  not decide which features take which policy.
- A failed reference leaves the previous body in place and flags the feature Error (F43 wording "using cached
  geometry"), rather than rolling the whole timeline back (E-state-machines rollback is not modelled).
- Signature tolerance is a fixed 1e-6 absolute; the real one must come from the document units/tolerance (G.6).

### Not covered

- Fuse/merge, revolve, pattern, mirror, chamfer, draft, hole features; non-planar splits; curved support faces for
  sketches; variable fillets.
- Kernel failure modes inside rebuild (only mapped to `KERNEL.OP.FAILED`), performance of rebuild at 50 features
  (roadmap M11 target < 1 s), cross-platform determinism of labels (K-4).
- The label strings are test-visible only; the H.3.9 `GeneratedLabel{feature, role, key, splitIndex}` shape is not
  produced yet.

## K-3 — profile detection (planar arrangement) · PASSED (2026-10-13)

Code: `poc/sketch-solver/src/profiles.rs` (`detect_profiles(&Sketch) -> ProfileSet`), tests `tests/profiles.rs` (18/18).
Same crate as K-1 (pure Rust, `libm`, wasm32 build OK, clippy `-D warnings` clean). K-1 suites and the golden
digest are unchanged.

### Algorithm

1. Non-construction curves → primitives (line, arc, circle). Zero-length lines and r ≤ tol are skipped with
   `SKETCH.GEOMETRY.DEGENERATE`; exact duplicates are dropped with `SKETCH.GEOMETRY.DUPLICATE` (one per extra copy).
2. Cut points per primitive: carrier intersections (line–line, line–arc, arc–arc, tangent within tol) filtered to both
   primitives, plus endpoints that lie within tol of another curve (T junctions, overlaps, near misses).
3. All cut points are clustered within `TOL_LIN` = 1e-5 mm into vertices (transitive; the first point in (x, y) order
   represents the cluster). The sketch's own points are never changed (F76).
4. Edges between consecutive cut points; identical sub-edges (overlapping curves) collapse.
5. Half-edge faces. Edges with the same face on both sides (dangling arms, bridges between nested loops) are removed
   until none is left; tangent curves at a vertex are ordered by curvature.
6. Counter-clockwise cycles are profiles; each clockwise cycle (a component's outer boundary) becomes a hole of the
   smallest profile of another component that contains it (winding number with exact circular-segment terms).
7. Areas by Green's theorem relative to the loop's first vertex, so 1e6 mm offsets do not cancel.

### Results

| Case | Result |
| --- | --- |
| SK-01 rectangle 40×20 | 1 profile, 800 mm² |
| SK-10 chain gap 1e-5 / 2e-5 (also 0, 5e-6, 1.6e-5) | 1 / 0 profiles; 0 and 5e-6 close, 1.6e-5 and 2e-5 do not |
| SK-11 side 2⁻¹⁰ and 1e6, offset 0 and 1e6 | area = stored side², rel 1e-12 |
| SK-12 rect ⊃ circle ⊃ circle | 3 profiles, loops 2 / 2 / 1, areas exact (rel 1e-12) |
| F74 overlapping circles | 3 regions: lens 122.84 + 2 crescents |
| F74 tangent circles | external → 2; internal → 2 (disc + crescent) |
| F74 bowtie | 2 triangles of 25 |
| F74 rectangle crossed by unconnected line | 2 halves of 50 |
| F74 overshooting tic-tac-toe | 1 inner square (16) |
| F75 duplicate circle / rectangle | 1 profile each + `SKETCH.GEOMETRY.DUPLICATE` |
| construction line across a rectangle; open chain | still 1 profile; 0 profiles |
| arcs: half disc, ¾ disc (sweep > π), slot | exact areas |
| bridge line between nested loops | 2 profiles, areas unchanged |
| property: 12 random edge-to-edge chords in a rectangle, 8 random chords in a disc, 20 seeds each | regions tile the area to 1e-9 |
| determinism / curve insertion order | identical result; same areas for reversed insertion |
| performance, 1020 curves → 400 profiles (release) | 13.5 ms (target < 100 ms) |

### Decisions to review

1. New code **`SKETCH.GEOMETRY.DUPLICATE`** (Warning) proposed for H.5 as F.8 asks; degenerate input reuses
   `SKETCH.GEOMETRY.DEGENERATE`.
2. **Vertex clustering is transitive**: a chain of points each within 1e-5 of the next merges even if the ends are
   further apart. Fine for sketches; it is a sharp edge when many points sit within the tolerance.
3. **Hole ownership** is by the smallest containing profile of another component. A component that touches no other
   (nested loops) is a hole; one connected through a bridge is not (the bridge is dropped), as the reference does.
4. Closure test is `gap ≤ 1e-5·(1+1e-9)`. The reference boundary is between 1e-5 and 2e-5, so this is conservative.

### Not covered / follow-ups

- Ellipse, spline and conic curves (not in the K-1 model); a profile with one arc of sweep exactly 2π is treated as a
  circle.
- Output pieces name the source curve and endpoints but not a parameter range; when one curve is cut into several
  pieces the K-2 labels need a `splitIndex` (H.3.9) — the pieces are in loop order, which is deterministic but not yet
  a stable key across edits.
- Near-parallel lines closer than 1e-5 are treated as coincident only at endpoints; a long sliver between two lines
  0.9e-5 apart whose endpoints are far from each other is not collapsed.
- Profile ↔ extrude hand-off (K-2 `ProfilePoint` list) is manual in the tests; a conversion `Profile → polygon with
  curve ids` for line-only loops would be the first step of the integration.

## K-4 — document-level determinism (2026-10-13)

`kernel-spike/det` (`det run | det compare`): one document is rebuilt from scratch N times — sketch commands
(rectangle + dims + Fix, width toggled ×10, second under-constrained rectangle dragged ×10) → solve → profile
detection → timeline (extrude → groove → fillet → shell, refs by label) → canonical report (97 lines).
`S` lines (solved coordinates as f64 bits, profile areas, labels, resolutions, counts) must be bit-identical;
`N` lines (OCCT volume, face areas, centroids) must agree to rel 1e-12 (TX-DET).

| Check | Result |
|---|---|
| Windows MSVC, 101 rebuilds, 1 thread | 0 mismatches |
| Windows, 105 rebuilds on 8 threads | 0 mismatches |
| Linux (WSL2 Ubuntu 22.04, gcc, OCCT 8.0.1 same vcpkg commit), 101 rebuilds | 0 mismatches |
| Linux, 105 rebuilds on 8 threads | 0 mismatches |
| Windows vs Linux report | 97 lines; strict 0 differences; numeric worst rel **0** (all bit-identical) |

Digest strict `b2ed073592a780e4`, numeric `c09c9a0c3828f3b6`; golden report `kernel-spike/det/golden/report.txt`.
Cost: ~27 ms per full rebuild incl. sketch (Linux release). Final body: 12 faces, 30 edges, volume 57157.08 mm³.

Linux OCCT setup (no sudo): vcpkg checkout at the same commit as Windows, overlay port with
`BUILD_MODULE_Visualization/ApplicationFramework=OFF` and `opengl` dependency removed, overlay triplet
`x64-linux-occt` adding user-space X11/GL headers (TKService still compiles via DataExchange), `zip` unpacked from
a .deb. Build ≈ 10 min. Run with `OCCT_ROOT=~/vcpkg/installed/x64-linux-occt` and `LD_LIBRARY_PATH=…/lib`.

Limits / not covered: one document, one compiler per OS (no macOS, no clang, no `-march=native` or fast-math
variants), single CPU model; OCCT parallel booleans not forced on (default thread pool); STEP text not compared
(header carries a timestamp); shapes were not round-tripped through the saved-package format yet.
