# F.11 Own 2D constraint solver — design for PoC K-1 (language-independent, [P])

Goal: a deterministic, explainable 2D geometric constraint solver meeting artifact F (SK-01…SK-19, S-07/S-16) and the
behaviour decisions N.2a. PlaneGCS is used only as a comparison oracle in tests.

## 1. Model
- Unknowns: point coordinates (x, y), circle/arc radius, (later) ellipse/spline parameters. Lines are point pairs;
  circles = centre point + radius; arcs = centre + radius + start/end angles (or 3 points, decide in PoC).
- Fixed points/curves are removed from the unknown vector (not penalised) → exact anchoring (F66 semantics as
  "fix" = remove DOF).
- Each constraint contributes one or more residual equations r_k(x) = 0 with analytic Jacobian rows, e.g.
  horizontal: y2 − y1; parallel: cross(d1, d2)/(|d1||d2|); distance: |p2 − p1| − d; tangent line–circle:
  signed distance(centre, line) − r; equal: |d1| − |d2|; symmetric (endpoint mirror, N.2a-2 recommended): p2 − mirror(p1).
- Dimensions: driving = residual; driven = computed value only (no residual).

## 2. Algorithm
1. **Decompose**: build the constraint graph, split into connected components; solve independently (speed + clear
   diagnostics per cluster).
2. **Rank analysis** at the current point: J (m × n) → QR with column pivoting (or SVD for small n):
   - DOF = n − rank(J) → reported per sketch and per entity (projection of the null space onto entity variables).
   - **Redundant** constraints: rows dependent on others (rank deficiency in rows) → `SKETCH.CONSTRAINT.REDUNDANT`
     with the minimal dependent set; offer "make driven" (reference GUI dialog F38).
3. **Solve**: Levenberg–Marquardt, minimal-change objective — minimise ‖x − x₀‖ subject to r(x) = 0 using a weighted
   least-norm Gauss–Newton step: Δx = −Jᵀ(JJᵀ + λI)⁻¹ r (moves as little as possible from the previous solution; this
   reproduces the reference's "move the later entity, preserve lengths" behaviour F61–F65 when weights favour the
   last-edited entities).
   - Weights: entities touched by the current edit get low weight (free to move); previously placed entities high
     weight → "first selected stays" (F62) and drag behaviour (F231).
   - Convergence: ‖r‖∞ < 1e-10 (internal mm), max 50 iterations; else `SKETCH.SOLVE.NONCONVERGENT`, previous solution kept.
4. **Conflict**: residual cannot be reduced (inconsistent system) → `SKETCH.CONSTRAINT.CONFLICT` / `SKETCH.SOLVE.CONFLICT`,
   name the smallest conflicting subset (deletion filter over the cluster's constraints), keep previous geometry (SK-05).
5. **Branch stability**: start from previous coordinates; reject a converged solution whose orientation-sensitive
   signs (triangle orientation, arc direction, tangent side) flipped unless no other root exists → never mirror-flip (F107).
6. **Degeneracy guard**: after solving, check zero-length lines, zero radius, collapsed angles → reject with
   `SKETCH.DIM.INVALID_VALUE` (S-07, S-11), never silently produce ≈0 geometry.

## 3. Determinism
- Fixed variable and constraint ordering by stable IDs; no hash-map iteration order in numerics; IEEE double with no
  fused-multiply-add differences across platforms (compile flags pinned); single-threaded per cluster.
- Same document + same edit sequence ⇒ bit-identical coordinates (TX-DET, SK-02, SK-16).

## 4. Interfaces (I.3 `ISketchSolver`)
```text
solve(snapshot: SketchSnapshot, edit: EditContext{changedIds, dragTarget?}) -> SolveResult {
  status: solved | under | fully | redundant | conflict | nonconvergent,
  coords: {pointId → (x,y)}, dof: { total, perEntity }, redundant: [constraintId[]], conflicts: [constraintId[]],
  iterations, residual }
```

## 5. PoC plan (K-1) — exit criteria
1. Port SK-01…SK-19 + constraint matrix cases (F60–F67) as data-driven tests (JSON sketches + expected results).
2. Implement H/V, coincident, point-on-curve, distance (H/V/aligned), parallel, perpendicular, collinear, equal, radius/
   diameter, concentric, tangent, midpoint, symmetric, angle, fix.
3. Exit: all tests pass; ≤ 50 ms for 500 entities / 600 constraints on reference hardware; DOF counts match
   hand-computed values on 20 sample sketches; results identical on Windows + Linux.
