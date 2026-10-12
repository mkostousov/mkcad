# F. Sketch and constraint-system specification (draft 1)

Status: draft, 2026-10-11. Source of evidence: `docs/STUDY.md` findings F1–F110 (reference build Autodesk Fusion
Personal 2705.1.30, Windows 10 22H2 VM, en-US, mm). Every statement carries a class tag:
**[O]** OBSERVED, **[D]** DOCUMENTED, **[I]** INFERRED, **[P]** PROPOSED, **[U]** UNKNOWN, and the finding IDs it rests on.
Nothing here describes the reference application's internals; reference behaviour is described only as seen
through its UI or public API.

Reliability note: one build, one OS, one locale, VM graphics (dimension text not drawn, F40). GUI pixel thresholds
were measured at one zoom level only. Findings marked API were obtained through the public scripting API and can
differ from GUI behaviour (F9 vs F36, F15 vs F38, F17/F18 vs F39) — the GUI path is authoritative for UX
requirements, the API path for data-model requirements.

---

## F.1 Coordinate model

| Topic | Requirement | Class / evidence |
|---|---|---|
| Sketch plane | A sketch is owned by exactly one planar support (origin plane, planar face, or construction plane). Creating a sketch requires picking the support; origin planes are offered when none is preselected. | [O] F31 → [P] |
| View on entry | Entering a sketch orients the camera normal to the plane ("look at"). Leaving does not restore the camera. | [O] F31, F98 → [P] keep, with a preference to disable |
| Local frame | 2D sketch coordinates (u,v) with origin at the projection of the component origin; mapping to model space is a rigid transform stored with the sketch. | [P] |
| Units | Internal length unit is fixed (reference API uses cm, [D]); the new product shall store SI metres or mm internally as an explicit, versioned document constant, never inferred from display units. Expressions keep the user's units verbatim ("2 in"). Unitless input in a length field = document units. | [D], [O] F20, F71 → [P] |
| Face attachment | A sketch on a face follows the face when upstream dimensions change. If the face disappears the sketch keeps its last plane (cached) and enters Warning state with a "plane lost" diagnostic; it is not deleted. | [O] F27, F28 → [P] |
| Projected geometry | Projected curves are reference + linked; on source loss they stay as cached geometry and the sketch goes to Warning "project source lost". | [O] F84 → [P] |
| Construction geometry | Construction curves participate in constraints but never in profiles. | [O] F78 → [P] |
| Stable IDs | Every point, curve, constraint and dimension has a document-unique, never-reused ID independent of creation order in the UI and of solver ordering. | [P] (reference IDs not observable) |
| Tolerance | Profile closure tolerance: reference closes a chain at end gap ≤ 1e-5 mm and not at ≥ 2e-5 mm. New product: a single documented linear tolerance `tol_lin` (proposed 1e-5 mm relative to a 1 m model box) used for closure, coincidence tests and degeneracy; never applied silently to merge points. | [O] F76 → [P] |
| Range | Profiles must be valid from 1e-3 mm to 1e6 mm feature size and at 1e6 mm offset from the origin. | [O] F77 → [P] acceptance bound |

## F.2 Geometry

### F.2.1 Entities [P] (reference tool list [O] F34)

| Entity | Parameters | Degeneracy rule |
|---|---|---|
| Point | (u,v) | — |
| Line segment | start point, end point (shared points) | Zero length: reference API accepts it (no profile) [O] F78; new product: reject in GUI, allow in API with Warning diagnostic [P] |
| Circle | centre point, radius | r ≤ 0 rejected ("invalid argument radius") [O] F78 |
| Arc | centre, start, end points, orientation | as circle |
| Ellipse / elliptical arc | centre, major axis point, minor radius | reference adds 2 construction axis lines + 4 Coincident [O] F130 |
| Spline (fit / control) | ordered points, degree | fit spline: no constraints; control spline: + construction control polygon [O] F130 |
| Conic | ends, shoulder, rho | single entity, no constraints [O] F130 |
| Slot (5 variants) | composed lines + arcs + constraints | straight: 2 lines + 2 arcs + construction centre line, 4 Tangent + Parallel (+ Coincident/MidPoint per variant); arc slots: 4 arcs + construction arc, 3 Tangent; no dims via API [O] F132 |
| Text | — | out of MVP scope [P] |

Composite tools create plain entities plus constraints:
- 2-point rectangle (GUI): 4 lines with shared corners + 2 Horizontal + 2 Vertical, no dimensions [O] F36. The API
  variant creates no H/V constraints [O] F9. **[P]** The new product's API and GUI shall produce identical results
  for the same command; constraints are part of the command's contract.
- Polygon (n sides): n lines + ONE n-ary Polygon constraint [O] F131. Centre and 3-point rectangles via API: 4 lines,
  no constraints (as F9) [O] F131; GUI variants expected to add H/V like F36 [I]. **[P]** Composite tools emit their
  constraints identically through GUI and API (N.2-7).

### F.2.2 Shared points [P]
A curve endpoint is a first-class point entity. Two curves are "connected" only when they reference the same point
entity (merge) or are tied by a Coincident constraint. The reference distinguishes these: GUI endpoint snap merges
(F88), snap to a circle centre adds a Coincident (F90). The new product shall keep the same two mechanisms and show
them distinctly in the learning mode.

## F.3 Constraints

DOF counts are mathematical facts for the parameterization above [P]. Movement policy column is the reference
behaviour when the constraint is added to unconstrained geometry [O] F61–F65, F109.

| Constraint | Inputs (order matters) | DOF removed | Reference movement on apply | Notes |
|---|---|---|---|---|
| Coincident pt–pt | P1, P2 | 2 | second entity translates rigidly [O] F64 | |
| Point-on-curve | P, C | 1 | the curve translates, not the point [O] F64 | snap ≤ ~2 px commits it [O] F89 |
| MidPoint | P, L | 2 | line translates [O] F64 | |
| Horizontal / Vertical | L | 1 | line rotates about its midpoint, length kept [O] F61 | |
| Parallel / Perpendicular | L1, L2 | 1 | L1 stays, L2 rotates about midpoint [O] F62 | |
| Collinear | L1, L2 | 2 | L2 translates onto L1's line; if L2 has an endpoint on L1's extension it rotates about that endpoint [O] F62, F109 | |
| Tangent (line–circle) | L, C | 1 | line translates [O] F64 | |
| Equal (lines) | L1, L2 | 1 | L2 keeps start, end moves [O] F63 | |
| Equal (circles) | C1, C2 | 1 | C2 radius changes [O] F63 | |
| Concentric | C1, C2 | 2 | C2 moves [O] F63 | |
| Symmetry | E1, E2, axis | 2 (lines) | E2 made collinear with mirror of E1's infinite line, own length kept; endpoints not mirrored [O] F65 | [P] new product: for lines, symmetric endpoints (4 DOF) is clearer for students — decide (Open question F-Q3) |
| Fix | entity | all | — ; stored as a curve property, not a constraint [O] F66 | [P] model Fix as a constraint record for uniform diagnostics |
| Polygon | n lines | n-ary (regularity) | created by the polygon tool as ONE constraint [O] F131 | |
| Circular / rectangular pattern | source entities + centre/directions | composite | ONE pattern constraint; live — source edits propagate, quantity is a parameter [O] F136 | |
| Curvature (G2) | 2 curves with a shared end point | 1–2 | applies as one Smooth constraint when the ends already coincide [O] F208 | later phase |
| Distance / H / V distance | 2 entities | 1 | first selected kept, opposite side moves [O] F37 | |
| Radius / diameter | C | 1 | [O] F108 (used) | |
| Angle | L1, L2 | 1 | free line rotates about the shared point but its LENGTH CHANGES (45°→30°: 28.3→27.8 mm); 0°/180°/−10° accepted and collapse the line to ≈0.1 mm, not restored when the angle returns [O] F134 | [P] keep length (rotate rigidly) and reject 0°/180° with `SKETCH.DIM.INVALID_VALUE` |
| Offset | chain → chain | composite | creates Offset constraint + driving offset dimension [O] F82 | |

Error classes on apply [O] F11, F15, F60, F66:
- `DUPLICATE` — identical constraint already exists ("already been applied"). No mutation.
- `OVER_CONSTRAINED` — redundant but consistent (redundant dimension, redundant Parallel). API: error; GUI (dimension):
  modal offering *Create Driven* or *Cancel* (F38).
- `SOLVE_FAILED` — inconsistent (e.g. Horizontal on a fixed oblique line). No mutation.

**[P]** Stable machine codes: `SKETCH.CONSTRAINT.DUPLICATE`, `SKETCH.CONSTRAINT.REDUNDANT`,
`SKETCH.CONSTRAINT.CONFLICT`, `SKETCH.SOLVE.NONCONVERGENT`, `SKETCH.DIM.INVALID_VALUE`. The reference merges
redundant constraints and redundant dimensions into one code (F15); the new product keeps them distinct per
offending entity so diagnostics can name them.

### F.3.1 Dimensions
- Placing a dimension first records the measured value at full precision ("41.7509298375 mm"), typing a value is a
  second undoable transaction [O] F51, F48.
- Driven (reference) dimension: displayed in parentheses, `isDriving=false`, updates with geometry [O] F38.
- Value validation, GUI: 0 and negative values for a length are refused in place (red underline, Enter does not
  commit, no undo entry) [O] F39, F50. API: 0 is stored but geometry does not follow (silent mismatch) [O] F17;
  negative flips the side [O] F18; impossible triangle sets are stored with geometry left at the last solution [O] F105.
- **[P]** The new product shall never store a dimension whose value disagrees with the solved geometry. API and GUI
  share one validator; rejected values return `SKETCH.DIM.INVALID_VALUE` or `SKETCH.SOLVE.CONFLICT` and leave the
  document unchanged. Signed distance is a separate, explicit option, not a side effect of a negative value.

## F.4 Inference and snapping

| Aspect | Reference behaviour | Class | Proposed requirement |
|---|---|---|---|
| Grid snap | Palette "Snap" on: points land on grid intersections; persists across documents in a session; when an inference is active the point is not grid-snapped | [O] F85, F92 | same; persist as user preference [P] |
| Horizontal/Vertical | cursor within ~9 px (screen) of horizontal from start; committed as constraint on click; independent of line length | [O] F87 (5/8 px yes, 10–14 px no); zoom-independent pixel threshold [O] F161 | threshold in screen px, preference, default 8 px [P] |
| Endpoint | ≤ ~8 px → point merged; 16 px → none | [O] F88 | merge radius 8 px [P] |
| Point on curve | ≤ 2 px → Coincident; 5 px → none | [O] F89 | 4 px [P] — reference value is very tight for students (Open question F-Q2) |
| Centre | near circle centre → Coincident (no merge) | [O] F90 | same |
| Tangent | end near tangent point → Coincident on circle + Tangent | [O] F91 | same |
| Midpoint | end ≤ ~2 px from a line midpoint → MidPoint; 9 px along the line → on-curve Coincident only | [O] F158 | midpoint radius 8 px [P] |
| Alignment | end aligns to an existing point's x/y without adding a constraint | [O] F162 | show dashed alignment guide; no constraint [P] |
| Competing | Parallel beat Horizontal; Coincident+Perpendicular beat Horizontal (also on construction/projected lines) | [O] F92, F160, rule [I]: relational > absolute | explicit ranked list [P]: endpoint > centre > midpoint > tangent > on-curve > parallel/perpendicular to recent > H/V > grid |
| Feedback | rubber band, length field (pre-selected), angle field relative to a reference line, "Specify next point" tooltip, H glyph near line, snap marker | [O] F86 | same set; glyphs must also have a text/ARIA equivalent [P] |
| Modifier suppression | Ctrl held during the move suppresses inference; Shift does not; modifiers only at click time do nothing | [O] F93, F163, F164 | same key (Ctrl/⌘), documented in the status bar while drawing [P] |
| Projected/construction targets | both are inference targets; construction endpoint → point merged; projected endpoint → Coincident constraint (projected points cannot be shared) | [O] F159, F165 | same; explain the difference in learning mode [P] |

## F.5 Solver lifecycle (state machine) [P], grounded in [O] where noted

```text
IDLE --tool start--> GEOMETRY_INPUT
GEOMETRY_INPUT --cursor move--> INFERENCE_PREVIEW (candidate set recomputed per move; no document mutation)
INFERENCE_PREVIEW --click--> CONSTRAINT_PENDING (new entities + inferred constraints in a transient changeset)
CONSTRAINT_PENDING --> SOLVING (incremental solve, initial guess = current coordinates)
SOLVING --converged, residual <= tol--> SOLVED --> classify:
        UNDER_CONSTRAINED (DOF > 0) | FULLY_CONSTRAINED (DOF = 0)
SOLVING --rank-deficient but consistent--> REDUNDANT  --> reject changeset (API) / offer driven (GUI dim)
SOLVING --inconsistent--> CONFLICTING --> reject changeset, restore pre-change state
SOLVING --iteration/time limit--> NON_CONVERGENT --> reject changeset, restore
any interactive state --Esc--> CANCELLED --> discard transient changeset --> IDLE
```

- Commit granularity: each committed segment/shape is one transaction [O] F48 (rectangle + its 4 constraints is
  one undo step). Esc discards only the rubber band; committed segments stay [O] F97. Enter commits and ends the
  tool [O] F96.
- Reference keeps the previous solution when a dimension edit fails (F11, F105) — matches CONFLICTING→restore.
- Limits [P]: ≤ 50 Newton/LM iterations, ≤ 50 ms per interactive solve for ≤ 500 entities; residual ≤ 1e-10
  (internal units²); exceeding returns NON_CONVERGENT.
- Determinism [P]: same document + same edit sequence ⇒ bit-identical coordinates. Reference: fully constrained
  sketches are deterministic (F16, F107); under-constrained ones are path-dependent (F10, F108). The new product
  may be path-dependent for under-constrained geometry (minimal-change is desirable, F67 [I]) but must be
  reproducible for the same sequence, and must display DOF so students see why.

## F.6 DOF and diagnostics

Reference exposes per-curve and per-sketch "fully constrained" flags [O] F14; no numeric DOF count in the Sketch
Palette or status bar [O] F141. Signalling is visual only [O]: under-constrained curves orange/amber, fully
constrained black (F224); browser sketch icon gets a red padlock when the whole sketch is fully constrained (F225);
timeline icon unchanged; free points drag with grid snap and add no constraints (F231). AutoConstrain (automatic
constraints + dimensions) is subscription-gated (F230); under a trial it offers several alternative complete
solutions with thumbnails, counts of added constraints/dimensions, a padlock for fully constraining results,
"Generate more", and a canvas preview; accepting one adds e.g. Coincident + Horizontal + Diameter/Offset/Linear dims
at current measured values (F234, F235). **[P]** Offer an equivalent "suggest constraints" tool in the learning mode
(free, explainable: each proposal lists which DOF it removes). **[P]** The new product shall:
1. Show the remaining DOF count per sketch and colour per entity (under / fully / over).
2. On REDUNDANT/CONFLICTING name the smallest set of constraints that participate (from the Jacobian rank analysis)
   and offer: delete one, convert dimension to driven.
3. Separate rigid-body DOF (sketch not anchored) from internal DOF in the learning mode.
4. Never move fully constrained geometry on drag/API move (reference returns success but restores, F19 → [P] return
   `SKETCH.MOVE.CONSTRAINED` instead of silent success).
5. Branch stability: when a dimension changes, prefer the solution nearest to the previous one; never mirror-flip
   (reference: no flip in F107).

## F.7 Edit operations (reference behaviour → requirement)

| Operation | Reference [O] | Requirement [P] |
|---|---|---|
| Trim | removes picked span to nearest intersections, adds Coincident at new ends; middle trim → 2 pieces + 2 Coincident + Collinear; no intersections → curve deleted (F79) | same |
| Extend | to next boundary, adds Coincident (F80) | same; return the changed curve |
| Break | 2 pieces sharing a point, 1 Coincident, no Collinear (F81) | add Collinear for consistency with Trim — decide (F-Q4) |
| Offset | new curves + Offset constraint + driving dimension; collapsing inward offset → "created no geometry" (F82) | same, code `SKETCH.OFFSET.EMPTY` |
| Fillet | trims both lines, arc + 2 Tangent; r = segment length consumes both lines (arc only, no constraints); r > length accepted and EXTENDS/reverses the lines beyond their far ends (F83, F135) | reject radius larger than the shorter adjacent segment (`SKETCH.DIM.INVALID_VALUE`) |
| Delete | deleting a curve removes constraints that reference it, keeps attached curves (F95) | same; one transaction |
| Pattern (circular/rectangular) | copies + ONE live pattern constraint; quantity is a parameter; source edits propagate (F136) | same |
| Move / copy | translation keeps constraints; ROTATION silently deletes H/V constraints of moved curves; copy is unconstrained; moving onto a copy leaves coincident duplicates (F137) | rotation that breaks H/V → warn and list removed constraints (`SKETCH.CONSTRAINT.REMOVED` [P]); duplicates flagged `SKETCH.GEOMETRY.DUPLICATE` |
| Scale | uniform only ("Only uniform scaling is supported"); scaling silently DELETES driving dimensions (F207) | keep dimensions and scale their values, or ask; never silent (`SKETCH.CONSTRAINT.REMOVED`) |
| Mirror | [U] GUI-only (no API equivalent exercised) | creates mirrored copies + Symmetry constraints [P] |
| Finish sketch | leaves edit mode, is an undo step, no profile check / warning for 0 profiles (F98, F99) | same; learning mode may hint "no closed profile" |

## F.8 Profiles

Reference [O] F73–F76: regions come from curve intersections, not connectivity (crossing unconnected line splits a
rectangle; overshooting lines bound an inner square); nested loops produce one profile per region with inner loops
as holes; exact duplicate curves are tolerated and yield one profile; open chain → 0 profiles; construction curves
excluded. **[P]** Same rules, implemented as a planar arrangement over all non-construction curves with tolerance
`tol_lin`; duplicate curves produce a Warning diagnostic `SKETCH.GEOMETRY.DUPLICATE` (not an error).

## F.9 Conformance tests (seed set; full suite in artifact M)

Each test: actions → expected logical state → UI feedback → tolerance. Tol = 1e-9 mm unless noted.

| ID | Actions | Expected | Ref |
|---|---|---|---|
| SK-01 | 2-pt rectangle from origin, add W=40, H=20 | 4 lines, 2H, 2V, fully constrained, 1 profile area 800 mm² | F14, F36, F68 |
| SK-02 | SK-01, then W 40↔73.3 ×20 | 1 distinct geometry | F16 |
| SK-03 | SK-01, add dimension on opposite edge (GUI) | modal offering driven; driven dim shows "(40.00 mm)"; API → REDUNDANT, no change | F38, F15 |
| SK-04 | SK-01, set W=0 / −10 | refused, document unchanged, no undo entry | F39 [P] for API |
| SK-05 | triangle 30/20/20 then 30/10/10 | CONFLICTING, previous geometry and expressions kept | F105 [P] |
| SK-06 | triangle 30/15/15 | solves collinear, Warning "degenerate" [P] | F106 |
| SK-07 | alternate side 20↔28 ×10 | apex stays y>0, 2 positions | F107 |
| SK-08 | apply each constraint of F.3 to unconstrained geometry | movement per table; lengths preserved where stated (tol 1e-6 mm) | F61–F65 |
| SK-09 | apply same constraint twice | DUPLICATE, no change | F60 |
| SK-10 | chain with end gap 1e-5 / 2e-5 mm | 1 / 0 profiles | F76 |
| SK-11 | squares 1e-3, 1e6, offset 1e6 mm | exact areas (rel tol 1e-12) | F77 |
| SK-12 | rect ⊃ circle ⊃ circle | 3 profiles, middle has 2 loops | F73 |
| SK-13 | Ctrl+Z ×5 after rect + dim + value + driven dim + sketch | reverse order exactly as F48, Ctrl+Y ×5 restores bit-identically | F48, F49 |
| SK-14 | GUI line end 5 px / 12 px off horizontal | H constraint / none (new product thresholds per F.4) | F87 |
| SK-15 | project edge, delete source | curve kept, sketch Warning `SKETCH.REF.LOST` | F84 |

## F.10 Open questions (sketch)

- F-Q1 Reference inference at other zoom levels; modifier held during move (needs harness `move --hold`).
- F-Q2 Thresholds for students: adopt reference tight values or larger defaults?
- F-Q3 Symmetry semantics for lines (reference: infinite-line mirror; proposal: endpoint mirror).
- F-Q4 Break vs Trim constraint consistency.
- F-Q5 Ellipse, spline, conic, slot, polygon parameterization and constraints — untested in reference.
- F-Q6 Whether the reference shows a numeric DOF count anywhere (not observed).
