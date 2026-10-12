# E. State-machine and transaction catalogue (draft 1)

Status: draft, 2026-10-11. Evidence: `docs/STUDY.md` (reference: Fusion Personal 2705.1.30, Windows 10 VM).
Tags: [O] OBSERVED, [D] DOCUMENTED, [I] INFERRED, [P] PROPOSED, [U] UNKNOWN. Sketch-internal states are in
artifact F §F.5; this file covers document-level commands.

## E.0 State partitions [P]

| Partition | Persisted in document | In undo history | Examples |
|---|---|---|---|
| Persistent document state | yes | yes | sketches, features, parameters, timeline order, suppression, marker position |
| Transient command state | no | no | open dialog values, preview body (F70), rubber band |
| Selection state | no | no [O] F72 (Cancel/OK clear selection; neither is an undo step) | preselection, selection set |
| View/camera | per-document view (optional) | no | orbit, look-at on sketch entry (F31) |
| User preferences | user profile | no | grid snap (session-persistent, F85), "do not show again" (F38) |
| Derived caches | regenerable | no | B-rep, tessellation, cached geometry for lost references (F27, F43, F84) |
| Undo history | session only [P] | — | — |
| Background jobs | no | no | export, tessellation |

Open question E-Q1: does the reference persist the timeline marker position on save? [U] (no document saved — licence limit).

## E.1 Transaction rules [P]

1. Every user command commits **at most one** transaction; Cancel commits none. Reference: Extrude Cancel → no
   timeline change [O] F72; rejected in-canvas edits and Esc → no undo entry [O] F50.
2. A transaction is atomic: either all mutations plus a successful recompute, or the pre-command state.
   Exception: *recoverable error state* — the feature is committed but marked Error/Warning with a structured
   diagnostic and cached geometry (reference [O] F27, F43, F84). The new product keeps this exception, but only for
   feature *recompute* failures caused by upstream edits, never for the command the user is executing (a failed
   new feature is rejected, not committed).
3. Interactive previews never mutate persistent state (reference preview body exists only while the dialog is open,
   no timeline item [O] F70).
4. Undo restores persistent state bit-identically; redo replays exactly [O] F49 → acceptance requirement.
5. Mode changes that the user perceives as steps are undo steps: Finish Sketch is undoable (re-enters sketch) [O] F99.

## E.2 Catalogue

Template fields not listed are "n/a". "Ref" gives the evidence finding.

### E.2.1 New design
- Trigger: File ▸ New Design / GUI chooser (Part, Assembly, Hybrid, Drawing, Electronics…) [O] F4. API default
  differs from GUI default (Hybrid vs Part) [O] F3 → [P] one default for both.
- Postconditions: empty document, root component, origin (hidden), named views TOP/FRONT/RIGHT/HOME, units mm/g [O] F104.
- Dirty: not dirty until first mutation; first sketch makes title "Untitled*" + unsaved banner [O] F32.
- Undo: not undoable (creates a new history) [P].
- Acceptance: new document has 0 timeline items, 0 bodies, dirty=false.

### E.2.2 Save / Save As / Autosave / Recover
- Reference [O] (X-16, F209–F215): save goes to a local cache then uploads; versions accumulate with save comments as
  descriptions; while uploading, re-open asks "Open locally saved file instead?"; persisted: timeline, marker
  position, parameters, named views; not persisted: camera, selection; automatic versioning and save-on-close off
  by default; a crash-recovery backup is written ≈1 min after an edit but after a force-kill was not offered on
  restart (single trial) — unsaved work lost.
- [P] Save writes a versioned package (artifact H); never blocks the UI thread for > 100 ms; atomic replace
  (write temp + rename). Autosave every N min to a recovery slot, separate from user file. Recover offers the
  newest recovery slot on next start after an unclean exit. Save is not an undo step and does not clear history.

### E.2.3 Create sketch
- Trigger: Create Sketch; precondition: a planar support picked (origin planes shown when nothing preselected) [O] F31.
- Mutation: adds Sketch to timeline + browser "Sketches" folder [O] F104; enters sketch edit (contextual SKETCH tab,
  camera look-at) [O] F31.
- Undo: one step; undo deletes the sketch and exits sketch mode [O] F48(5).

### E.2.4 Edit sketch / Finish sketch
- Enter: timeline item context menu "Edit Sketch" [O] F101 (double-click also, unreliable in harness).
- Finish: active edit object → root component, SKETCH tab removed, camera unchanged, 0 profiles allowed without
  warning [O] F98. Undo step [O] F99. Esc with no command does not exit sketch [O] F100.
- Recompute: on finish (or on each solve [P]) downstream features recompute [O] F12.

### E.2.5 Create feature (Extrude as template)
```text
IDLE --Extrude--> DIALOG_OPEN(profile preselected? → 1 selected) [O] F69
DIALOG_OPEN --value valid--> PREVIEW (transient body, no timeline item) [O] F70
PREVIEW --value 0--> DIALOG_OPEN (OK disabled) [O] F69
PREVIEW --OK / Enter in field--> COMMIT: timeline +1 feature, selection cleared [O] F72
any --Cancel/Esc--> IDLE: no mutation, selection cleared [O] F72
COMMIT --kernel failure--> [P] stay in DIALOG_OPEN with diagnostic; no commit
```
- Parameters (reference defaults [O] F69): type solid|thin, profiles, start = profile plane, direction one side,
  extent distance, distance 0 mm, taper 0°, operation New Body (F143). The operation then switches automatically
  with the preview: overlap with a body → Cut (+ "Objects To Cut" input), face contact only → Join, else New Body
  (F144; rule [I] F145; F113 was a post-entry state). [P] Same auto rule until the user picks an operation
  explicitly (then it sticks for the command); chosen operation shown in the preview colour and announced.
- Acceptance: Cancel leaves timeline count, body count and undo depth unchanged.
- Generalisation to other features [O] F166: every feature dialog opens with OK disabled until its required
  selections exist; Measure is modeless with Close instead of OK/Cancel. Creation failures are atomic — no
  feature, no timeline item, body unchanged — with a coded kernel message plus a remedy hint (F169, F171).
  Reference *silent* outcomes the new product must diagnose: rolled-over fillet larger than the part (F168),
  taper/draft clamped at the apex (F177, F179), overlapping pattern/mirror bodies (F173, F174), sweep profile
  away from its path (F180) → `FEATURE.RESULT.TOPOLOGY_CHANGED`, `.CLAMPED`, `.OVERLAP`, `FEATURE.INPUT.DISJOINT`.

### E.2.6 Edit earlier feature / sketch → downstream rebuild
- Reference [O] F12, F28, F57: dimension/parameter edits propagate to downstream features; face-attached sketches
  follow their face.
- Surprises recorded [O]: F25 (fillet result split into 2 bodies without warning), F26 (cut off the face removes
  nothing, still Healthy). [P] New product emits Warning diagnostics `FEATURE.RESULT.TOPOLOGY_CHANGED` (body count
  changed) and `FEATURE.RESULT.NO_EFFECT` (boolean removed/added nothing).

### E.2.7 Suppress / unsuppress
- [O] F23: health "Suppressed", faces removed; unsuppress restores identical volume. [P] Undoable; suppression is
  persistent state; dependents recompute as if feature absent.

### E.2.8 Rollback (timeline marker)
- [O] F22: rolledBack flags on later items, model shows state at marker; move-to-end restores. Context menu entry
  "Roll Timeline Marker Here" [O] F101. New features inserted at the marker [O] F41.
- [O] F111: a marker move (API-driven) is its own undo step, independent of feature creation.
- [P] Marker position is persistent and moving it is an undo step (matches reference).

### E.2.9 Reorder
- [O] F24: independent reorder allowed with identical result; dependent reorder rejected with
  "CIRCULAR_DEPENDENCY …". [P] Code `GRAPH.REORDER.DEPENDENCY`; no mutation on rejection.

### E.2.10 Delete feature
- [O] F27: deleting a base feature also deleted dependent features (fillet, cut) but kept a face-attached sketch in
  Warning. [P] Before deleting, list dependents and ask: delete dependents / keep dependents (they enter Error with
  cached geometry) / cancel. One transaction.

### E.2.11 Missing / changed references
- [O] F41–F44: reference survives unrelated upstream insertion; a split edge maps to both pieces; a removed edge →
  Error "Edge 1 missing — 1 Reference Failures … using cached geometry"; removing the cause → automatic recovery.
- [P] Recoverable error state with diagnostic `REF.LOST` {feature, role, cached geometry}; automatic re-resolution on
  every recompute; user repair = edit feature and reselect. See artifact G for the reference model.

### E.2.12 Parameters
- [O] F52–F59: expressions with units; ";" as function separator in en-US; invalid / circular expressions rejected,
  previous expression kept; deleting a used parameter returns false; model parameters dN share one counter with
  user parameters.
- [P] Locale-independent expression grammar (separator ","; locale only affects display); codes
  `PARAM.NAME.INVALID` with reason {reserved_unit, syntax, duplicate}, `PARAM.EXPR.INVALID`, `PARAM.EXPR.CYCLE`,
  `PARAM.IN_USE`. The reference collapses all name errors into one message (F54) → [P] give the reason.

### E.2.13 Undo / redo
- [O] F48–F50: granularity = one committed user action (shape incl. its constraints, dimension placement, value
  edit, sketch creation); exact reverse replay.
- [O] F112: inside an open Extrude dialog Ctrl+Z first reverts the dialog value (preview removed), the next press
  cancels the command; only a further press reaches document history.
- [P] Same layered undo: in-command steps → cancel command → document history. The cancel step must equal Cancel
  (no mutation, TX-01).

### E.2.14 Export
- [O] F45–F47: STEP AP214 default, STL binary, 3MF OPC, OBJ+MTL; IGES/SAT blocked by licence.
- [P] Export is a background job (artifact H job contract): no document mutation, cancellable, progress events,
  result {path, format, schema, warnings}. Not an undo step; does not change dirty state.

### E.2.15 Import, unit change, long-running ops, tessellation
- Import [O]: STEP/SMT/F3D `importToTarget` into a parametric design creates a new occurrence (component) holding the
  body (F126); import to a new document gives a parametric design with the body in the root and one timeline item
  "Base Feature" (F127); F3D into a direct-modelling design fails (F128); API import options: DXF2D, F3D, IGES, SAT,
  SMT, STEP, SVG — no mesh import options (F129); SVG units read as 96-dpi px (F193). [P] Import is a job (H.4.8)
  producing one timeline item "Import <file>" with a body per solid; units preserved; SVG physical units respected.
- Unit change [O] F182–F183: expressions unchanged, model parameters display in document unit, user parameters keep
  theirs; unitless entries get the current unit appended at entry.
- [P] Document unit change only alters display/default units; stored expressions keep their units (F20, F71).
  Long operations > 500 ms show progress and are cancellable; cancellation restores the pre-command state.
  Tessellation runs in the background against an immutable geometry snapshot; the viewport shows the previous
  tessellation until replaced.

### E.2.16 Drawings (first production release) — reference [O] F216, F238–F245
```text
IDLE --Create Drawing(saved design)--> DRAWING_SETUP (standard, units, sheet size/count, automatic|manual)
DRAWING_SETUP --OK--> GENERATING (background job; reference uses a helper process, F236) --done--> SHEET_OPEN
SHEET_OPEN --Base View--> PLACE_BASE (dialog: orientation, style, scale, edge visibility) --click+OK--> SHEET_OPEN
SHEET_OPEN --Projected View--> PICK_PARENT --click(s)--> PLACE_PROJECTED* --Enter--> SHEET_OPEN
SHEET_OPEN --Auto Dimension--> CHOOSE_SCHEME (alternatives with previews) --OK--> SHEET_OPEN
SHEET_OPEN --Export PDF|DXF|DWG--> EXPORT_JOB (H.4.8) --> SHEET_OPEN
```
- [P] Generation failure is never silent (reference: helper crash left no message, F217/F236) → `DRAWING.GENERATE.FAILED`.
- [P] Projection angle comes from the sheet standard (ISO first angle, ANSI third angle) and is shown in the
  sheet settings (reference exposes none, F247; placed top view above front on ISO, F242).
- [P] Views are associative: model edits mark views stale; "Refresh" recomputes; dimensions follow (DR-03).

## E.3 Acceptance tests (document level)

| ID | Procedure | Pass criterion | Ref |
|---|---|---|---|
| TX-01 | Open Extrude, enter 10, Cancel | timeline, bodies, undo depth unchanged | F72 |
| TX-02 | Extrude 10 + Enter | +1 timeline item, selection empty | F72 |
| TX-03 | Insert hole before fillet via rollback | fillet Healthy, same edge | F41 |
| TX-04 | Upstream notch removes filleted edge, then delete notch | Error with REF.LOST, then Healthy with identical volume | F43, F44 |
| TX-05 | Reorder dependent feature before its parent | rejected, no change | F24 |
| TX-06 | Suppress / unsuppress fillet | identical volume (rel tol 1e-12) | F23 |
| TX-07 | Delete base feature | dependents prompt (P); no orphan without diagnostic | F27 |
| TX-08 | Finish sketch then undo | sketch edit re-entered | F99 |
| TX-09 | Circular parameter A=B, B=A | second assignment rejected, A unchanged | F55 |
| TX-10 | Export STEP | no dirty-state change; header schema AP214 or configured AP242 | F46 |

## E.4 Open questions
- E-Q1 Reference: persistence of marker, camera, selection on save/reopen.
- E-Q2 Answered for API moves (F111); GUI marker drag still to confirm.
- E-Q3 Answered (F112).
- E-Q4 Reference autosave/recovery behaviour (needs a saved document — costs 1 of 10 slots).
- E-Q5 Import behaviour (STEP/STL into a new document).
