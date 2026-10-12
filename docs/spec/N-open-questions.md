# N. Open questions and required experiments (draft 1)

Status: 2026-10-11. Consolidates the open questions of artifacts E, F, G and CD plus `STUDY.md` gaps.
Priority: P1 = blocks a spec decision, P2 = improves a requirement, P3 = completeness.

## N.1 Reference-behaviour experiments (Fusion VM)

| ID | Question | Method (cheapest first) | Prio | From |
|---|---|---|---|---|
| X-01 | Which piece does a split-face profile resolve to (F116)? 4 cases run (F117): not area, not circle containment, not a fixed corner. **CLOSED 2026-10-11 (F119):** the choice varies between identical runs; sketch origin, axes and `pointOnFace` are constant → not reproducible from visible inputs. Do not mimic; see N.2-4 | API job | P1 (decision N.2-4 does not depend on it) | G-Q2 |
| X-02 | Face references when the face is split upstream. **CLOSED 2026-10-11:** Shell (F118), Offset Face (F121), Draft (F122) keep both pieces; fillet with face input re-evaluates all edges of the pieces (F123). Only profile-from-face (F116/F119) picks one piece | API job, pattern of face_split_shell.py | P2 | G-Q2 |
| X-03 | Split edge later re-merged (reference set shrinks back?) **CLOSED 2026-10-11 (F124):** yes, reversible on suppress/delete and re-splits on any new groove | API: groove then delete groove (cf. F44) | P2 | G-Q1 |
| X-04 | GUI timeline marker drag / "Roll Timeline Marker Here" → undo step? **CLOSED 2026-10-11 (F150):** yes for both | GUI + doc_state (F111 covered API only) | P2 | E-Q2 |
| X-05 | Inference threshold at other zoom levels; modifier held during move. **Zoom part CLOSED (F161: pixel threshold, zoom-independent)**; modifier at click does not suppress (F163); **CLOSED (F164): Ctrl held during move suppresses inference, Shift does not** | harness `move --hold` + gline.ps1 at 2 zooms | P2 | F-Q1 |
| X-06 | Midpoint snap on a clean line; inference to projected and construction geometry. **CLOSED (F158–F160, F165):** construction endpoints merge, projected endpoints get Coincident; both give Perpendicular/Coincident on the curve body | gline.ps1 | P2 | F.4 |
| X-07 | Numeric DOF display anywhere in the UI. **CLOSED 2026-10-11 (F141):** none in palette or status bar → new product's DOF panel (F.6) is an improvement, not parity | OCR of sketch palette / status bar while sketch under-constrained | P2 | F-Q6 |
| X-08 | Ellipse, spline, conic, slot, polygon: created entities + constraints (API and GUI) | API job + read_sketch | P2 | F-Q5 |
| X-09 | Angle dimension behaviour, Curvature and Polygon constraints | constraint_matrix extension | P3 | F.3 |
| X-10 | Oversize sketch fillet result geometry (F83) | API: inspect curves after 50 mm fillet | P3 | F.7 |
| X-11 | Mirror, pattern, move/copy, scale in sketches | API job | P3 | F.7 |
| X-12 | Selection priority under the cursor (vertex/edge/face/body). **CLOSED 2026-10-11 (F147, F148):** vertex > edge > face; vertex band ≈ 8–12 px, edge band ≈ 6–8 px | GUI hover + status bar OCR | P2 | G-Q3 |
| X-13 | Browser context menus (body, sketch, origin); Viewports ▾; ViewCube "Set current view as ▸". **MOSTLY CLOSED (F151–F157):** browser, ViewCube, Grid/Snaps, Display Settings menus done; Viewports ▾ not captured | right-click + OCR | P3 | CD gaps |
| X-14 | Remaining ribbon menus (SELECT panels etc.) | tools/menus.ps1 | P3 | CD gaps |
| X-15 | Import STEP/STL into a new document: result kind, timeline item, units | API importManager | P2 | E-Q5 |
| X-16 | Save / autosave / recovery, persistence of marker, camera, selection. **CLOSED 2026-10-11 (F209–F215)**; drawings attempted, generation never completed (F216–F217) — drawing behaviour must come from public documentation [D] instead | costs 1 of 10 document slots — needs user OK | P2 | E-Q1, E-Q4 |
| X-17 | Is "L" the shortcut that opens Line (only menu labels verified for most letters). **CLOSED 2026-10-11 (F138–F140):** L, R, C, D, T, O, P, M verified in sketch; A/E/Q/F run solid commands from a sketch | press key in sketch, read activeCommand | P3 | CD |
| X-18 | Extrude default operation rule beyond F113 (touching an edge only; several bodies; Cut when extruding into a body?). **CLOSED 2026-10-11 (F143–F145):** New Body at d=0; auto Cut on overlap, Join on face contact, else New Body | API selection + GUI E + OCR | P3 | E.2.5 |

## N.2a Decisions taken (user, 2026-10-12)

| # | Decision | Notes |
|---|---|---|
| 1 | Inference thresholds: use the reference's values as defaults (8 px endpoint/midpoint, ~9 px H/V, 2 px on-curve), user-configurable; no separate learner defaults for now | user: "not following — skip"; treated as non-blocking default |
| 2 | Line symmetry = TRUE MIRROR: both endpoints mirrored about the axis (differs from reference F65) | user, option 2 |
| 3 | Break: pieces are NOT constrained to each other (matches reference: 2 Coincident to the cutting curves, no Collinear, F248). Trim keeps the reference's Collinear between remaining pieces of the same line | user: "parts are not constrained to each other — check it" → verified F248 |
| 4 | Split face/region references expand to ALL pieces; Warning `REF.AMBIGUOUS` (naming the pieces) when a feature needs exactly one | user, option 1 |
| 5 | Solved sketch coordinates are persisted as authoritative state and used as the solver's starting point | user, option 1 |
| 6 | Expression argument separator "," (locale-independent) | user: "," |
| 7 | API and GUI behave identically; the API is exposed to Python (scripting); GUI for now; no separate "teachability" constraints on the API | user |

## N.2 Product decisions needed (feed the "decisions before implementation" list)

1. Inference thresholds for learners — reference values (8 px endpoint, ~9 px H/V, 2 px on-curve) or larger defaults (F-Q2).
2. Symmetry semantics for lines (infinite-line mirror as reference, or endpoint mirror) (F-Q3).
3. Break vs Trim: add Collinear after Break? (F-Q4).
4. Split-region references: expand to all pieces vs Warning `REF.AMBIGUOUS` (G.4).
5. Persist solved coordinates as authoritative sketch state (G.1) — recommended yes.
6. Expression grammar locale-independence (separator ","), departing from the reference's ";" (E.2.12).
7. Whether API and GUI may differ (reference differs in F9/F36, F15/F38, F17/F39) — recommended: never.

## N.3 Proof-of-concept experiments (new product) — preliminary

| ID | PoC | Exit criterion |
|---|---|---|
| K-1 | PlaneGCS (or libslvs) behind a solver port: run SK-01…SK-13 of artifact F | all pass; ≤ 50 ms for 500 entities |
| K-2 | OCCT operation history → generated-topology labels for extrude + boolean + fillet; replay TX-03/TX-04 and the F116 case | references resolve as G.4 specifies |
| K-3 | Planar arrangement profile detection with tol 1e-5 mm: SK-10…SK-12 + F74 cases | exact region counts and areas |
| K-4 | Deterministic rebuild: same document rebuilt 100× on 2 OSes | bit-identical solved coordinates and volume to 1e-12 rel |
