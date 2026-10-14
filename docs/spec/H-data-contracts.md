# H. Data contracts and versioning rules (draft 1)

Status: draft, 2026-10-11. Tags: [O] observed in reference, [D] documented, [I] inferred, [P] proposed, [U] unknown.
Evidence IDs (Fnn) refer to `docs/STUDY.md`. Everything in this artifact is **[P]** unless tagged otherwise: the
reference's file format is not studied (clean-room; native `.f3d` internals are out of scope by design).
Contracts are illustrative (JSON-Schema-like pseudo notation), not production code.

## H.0 Conventions

| Topic | Rule |
|---|---|
| Identifiers | `StableId` = 128-bit random (UUIDv4/v7 text). Never reused, never derived from enumeration order. Sketch-local IDs (`p:…`, `c:…`, `k:…`, `d:…`) are unique within a sketch. |
| Numbers | IEEE-754 double. Lengths stored in **mm**, angles in **rad** (reference stores rad, F52; reference API uses cm — not adopted). |
| Expressions | Stored as written, with units (`"2 in"`, `"Len / 2"`) — reference keeps written units [O] F20, F52. Value is derived, cached. |
| Enumerations | Lower-case strings, closed sets per schema version; unknown values → document opens read-only for that entity (forward-compat rule H.6). |
| Diagnostics | Machine code + params; display text is localised and never parsed (brief §6). Catalogue H.5. |
| Schema version | Semantic: `major.minor`. Minor = additive fields only. Major = migration required. Every persisted object carries the document-level `schema` only (no per-object versions) except plugins (H.4.9). |

## H.1 Project package [P]

Decision matrix (brief §5 "single DB vs ZIP vs directory vs hybrid"):

| Criterion (weight) | Single SQLite | ZIP package | Directory bundle | Hybrid ZIP (chosen) |
|---|---|---|---|---|
| Single-file sharing in classrooms (3) | 3 | 3 | 1 | 3 |
| Diff-ability / inspection (2) | 1 | 2 | 3 | 2 |
| Partial recovery after corruption (3) | 2 | 2 | 3 | 3 (per-part CRC, H.6) |
| Cache separation (authoritative vs regenerable) (3) | 2 | 3 | 3 | 3 |
| Atomic save (2) | 3 | 2 (temp + rename) | 1 | 2 |
| Tooling availability (1) | 3 | 3 | 3 | 3 |
| **Weighted total** | 31 | 35 | 32 | **38** |

Chosen layout — OPC-like ZIP, extension `.mkcad` (name PROPOSED). Reference interchange 3MF is an OPC package [O] F47,
which shows the pattern is familiar to downstream tools.

```text
project.mkcad (ZIP, deflate; mimetype entry first, stored)
├── mimetype                         "application/vnd.mkcad.project+zip"
├── manifest.json                    schema, app version, parts list with sha256 + role
├── document/
│   ├── document.json                H.4.1 (authoritative)
│   ├── parameters.json              H.3.6
│   ├── sketches/<sketchId>.json     H.4.2
│   └── features/<featureId>.json    H.4.3
├── cache/                           regenerable — may be deleted at any time
│   ├── brep/<featureId>.brep        kernel-native B-rep per feature output (opaque, versioned by kernel id)
│   ├── mesh/<bodyLabel>.glb         tessellation for fast open
│   └── recompute.json               last recompute result (H.4.4) for diagnostics display
├── thumbnails/thumb.png
├── assets/                          textures, inserted images/SVG/DXF sources
└── history/migrations.json          applied migrations (H.6)
```

Rules: (1) opening a package whose `cache/` is missing or stale (kernel id/version differs) must rebuild from
`document/` and give the same result within tolerance (determinism, artifact M TX-DET); (2) the kernel's own
serialization is never the sole source of truth (brief §5); (3) save = write new ZIP to temp file, fsync, rename.

## H.2 Application-level contracts

| Contract | Persisted? | Owner | Notes |
|---|---|---|---|
| ApplicationState (open docs, active doc, workspace, layout) | user profile only | UI | Layout persists between sessions [P]; reference: palettes dockable [O] F101 |
| UserPreferences (units default, inference thresholds, learning mode) | user profile | UI | N.2-1 thresholds |
| ViewState (camera, named views) | document (named views) / session (camera) | UI | Reference has Named Views TOP/FRONT/RIGHT/HOME [O] F104 |
| SelectionState | never | UI | not undoable, cleared by OK/Cancel [O] F72 |
| BackgroundJob state | never | jobs | H.4.8 |

## H.3 Domain schemas

### H.3.1 Document metadata
```jsonc
Document {
  schema: "1.0", id: StableId, name: string,
  kind: "part" | "assembly" | "hybrid",            // reference kinds [O] F4; API default hybrid, GUI default part F3
  units: { length: "mm"|"cm"|"m"|"in"|"ft", mass: "g"|"kg"|"lb", angle: "deg"|"rad" },  // display units; storage fixed (H.0)
  tolerances: { modeling_mm: 1e-6, sketch_mm: 1e-5, angular_rad: 1e-9 },              // K-3/K-4 to confirm
  created, modified: ISO-8601, appVersion: string,
  rootComponent: StableId, timeline: Timeline, namedViews: NamedView[]
}
```

### H.3.2 Component and body hierarchy
```jsonc
Component { id, name, origin: OriginRef, sketches: StableId[], constructions: StableId[],
            bodies: BodyRef[], occurrences: Occurrence[] /* later phase */, appearance?: AppearanceRef }
BodyRef   { label: GeneratedLabel, name: string, visible: bool, material?: MaterialRef }   // body geometry = cache
```

### H.3.3 Feature dependency graph
```jsonc
Timeline { items: StableId[] /* feature + sketch + construction ids in order */, marker: int /* index after last active */ }
// Edges are derived from FeatureNode.inputs and expression references; not stored separately (avoid drift).
```
Reorder validity = topological-order check against derived edges; violation → `GRAPH.REORDER.DEPENDENCY` [O] F24.

### H.3.4 Sketch geometry
```jsonc
Sketch {
  id, name, plane: ReferenceDescriptor /* origin plane, construction plane or planar face */,
  frame: { origin: [x,y,z], xAxis: [..], yAxis: [..] },   // resolved, cached + used if support lost (F27 behaviour)
  points:  { id: "p:…", x, y, fixed: bool }[],             // solved coordinates are AUTHORITATIVE (G.1, F10)
  curves:  { id: "c:…", kind: "line"|"arc"|"circle"|"ellipse"|"spline"|"conic"|"point",
             construction: bool, refs: { start?: "p:…", end?: "p:…", center?: "p:…" },
             params: { radius?, majorRadius?, … }, projectedFrom?: ReferenceDescriptor }[],
  constraints: Constraint[], dimensions: Dimension[], textEntities: Text[]
}
```
Shared endpoints are point identity (one `p:` id), not a Coincident constraint — reference rectangle lines share
points with zero Coincident constraints [O] F9, F36.

### H.3.5 Sketch constraints and dimensions
```jsonc
Constraint { id: "k:…", kind: "horizontal"|"vertical"|"parallel"|"perpendicular"|"collinear"|"tangent"|"equal"
             |"coincident"|"midpoint"|"concentric"|"symmetric"|"curvature"|"polygon",
             entities: ("p:…"|"c:…")[] /* ordered: order matters for movement policy F62–F64 */ }
Dimension  { id: "d:…", kind: "distance"|"horizontal"|"vertical"|"radius"|"diameter"|"angle",
             entities: [...], driving: bool, expression: string /* driving only */,
             measured: number /* driven only, cache */, placement: [x,y] }
```
`fixed` is a point/curve flag, not a constraint — mirrors reference [O] F66 and simplifies DOF accounting.

### H.3.6 Parameter table and expressions
```jsonc
Parameter { id, name: Identifier, kind: "user"|"model", expression: string, unitKind: "length"|"angle"|"unitless",
            comment?: string, ownerDimension?: "d:…" }
```
Name rules [P]: `[A-Za-z_][A-Za-z0-9_]*`, case-sensitive, unique per document across user + model parameters (one
name space — reference shares the dN counter [O] F59), must not equal a unit symbol (reference rejects `A`, `W`
[O] F54) → `PARAM.NAME.INVALID` with `reason: "syntax"|"duplicate"|"reserved_unit"` (reference uses one generic
message for all [O] F54 — improve). Grammar decision N.2-6 (argument separator). Constants: `PI` and `E`
case-sensitive; unknown identifiers are errors (reference evaluates lower-case `pi` silently to 0 [O] F56 — not adopted).

### H.3.7 Materials and appearance
```jsonc
MaterialRef   { library: string, id: string, overrides?: { density_kg_m3?: number } }
AppearanceRef { library: string, id: string, overrides?: { color?: [r,g,b], texture?: assetPath } }
// assignment target: component, body label, or face label (face-level appearance later phase)
// Rules [O]→[P]: default material is a document setting (reference: Steel 7.85 g/cm³, F185); appearance derives from
// material unless an explicit AppearanceRef override exists; overrides survive material changes (F186, F187).
```

### H.3.8 Units and tolerances — see H.3.1; tolerance values are inputs to PoC K-3/K-4.

### H.3.9 Selection reference / ReferenceDescriptor (persistent references, G.4)
```jsonc
ReferenceDescriptor {
  kind: "face"|"edge"|"vertex"|"body"|"profile"|"plane"|"axis"|"point"|"sketchCurve",
  label: GeneratedLabel | null,             // e.g. { feature: "f-12", role: "side", key: "c:7" } / "cap:end"
  signature: { type: "plane"|"cylinder"|"line"|"circle"|…, centroid: [..], normal?: [..], size?: number },
  context: { featureAtSelection: StableId, timelineIndex: int }
}
GeneratedLabel { feature: StableId, role: string, key: string, splitIndex?: int[] }
```

### H.3.10 Camera and viewport state
```jsonc
Camera { kind: "perspective"|"orthographic", eye: [..], target: [..], up: [..], fovDeg?: number, extent?: number }
NamedView { id, name, camera: Camera, builtin: bool }    // TOP/FRONT/RIGHT/HOME builtin [O] F104
```

### H.3.11 Undo/redo command
```jsonc
UndoEntry { id, label: string /* "Extrude1", "Edit dimension d3" */, kind: "document"|"marker"|"sketch",
            before: Patch, after: Patch /* JSON-Patch over document/ parts */, groupId?: StableId }
```
Granularity rules [O]→[P]: one entry per committed user action; dimension creation and value edit are separate
entries (F48, F51); rejected inputs create none (F50); marker moves are entries (F111); in-command (dialog) undo is
separate from document undo (F112). Selection and camera never enter the stack (F72).

### H.3.12 Rebuild diagnostics — H.4.6 and catalogue H.5.

### H.3.13 Import/export jobs — H.4.8.

### H.3.14 Plugin manifest — H.4.9.

### H.3.18 Sketch operation log and provenance (internal now, UI later — user request 2026-10-12)
```jsonc
// on every sketch entity (points, curves, constraints, dimensions):
Provenance { createdBy: OpId, groupId?: GroupId /* composite tool result: rectangle, slot, polygon, pattern, import */,
             modifiedBy: OpId[] /* bounded, last N */ }
SketchOp { id: OpId, seq: int, kind: "line"|"rectangle"|"circle"|"trim"|"break"|"extend"|"offset"|"constraint"|
           "dimension"|"edit_dimension"|"drag"|"delete"|"import"|"auto_constrain"|..., params: {...},
           created: EntityId[], modified: EntityId[], deleted: EntityId[], groupId?: GroupId,
           diagnostics: Diagnostic[], at: ISO-8601 }
SketchOpLog { sketchId, ops: SketchOp[] }   // persisted with the sketch, INFORMATIONAL (not authoritative; N.2a-5)
```
Rules [P]: every sketch command emits exactly one SketchOp (same granularity as undo, F48); undo removes/re-adds the
op; the log survives Finish Sketch and save/open (reference loses it on Finish, F98); composite tools set one groupId
on all created entities and constraints. Later UI: sketch history panel + structure tree with per-entity DOF (F11 §4).
Replayable sketch history is a separate later decision; this contract supports it.

### H.3.16 Sheet-metal rule (later phase)
```jsonc
SheetMetalRule { id, name: "Steel (mm)", source: "library" | "document",
                 thickness: "2.5 mm", kFactor: "0.44", bendRadius: "Thickness" /* expression may reference thickness */,
                 relief: { shape: "straight" | "round" | "tear", width?: string, depth?: string },
                 cornerRelief2Bend: "trim_to_bend" | "round" | "square" }
// Reference defaults [O] F202, F204; body → ruleId; flat pattern is cache (G.7a).
```

### H.3.17 Drawing (first production release)
```jsonc
DrawingDocument { schema, id, source: { documentId, version /* pinned design version */ },
  standard: "ISO" | "ANSI", projection: "first" | "third" /* derived from standard unless overridden */, units: "mm"|"in",
  textStyle: { font: "Arial", heightGroup: "medium" }, lineWidthGroup: "medium",       // reference defaults F247
  sheets: Sheet[], titleBlock: { template, fields: { title, author, date, partNumber, ... } } }
Sheet { id, name: "Sheet1", size: "A3", widthMm: 420, heightMm: 297, views: View[], annotations: Annotation[] }
View  { id, kind: "base"|"projected"|"section"|"detail"|"auxiliary"|"break"|"crop",
        parent?: viewId, orientation: "front"|"top"|..., scale: "2:1", style: "visible"|"visible+hidden"|"shaded"|...,
        position: [x,y] /* sheet mm */, edgeVisibility: { tangent, interference, thread }, stale: bool }
Annotation { id, kind: "dimension"|"note"|"centerMark"|"centerLine"|"symbol", view?: viewId,
             references: ReferenceDescriptor[] /* model edges via G.4 labels → associativity */, value?: string }
// Diagnostic: DRAWING.GENERATE.FAILED (error); exports via H.4.8 jobs (pdf, dxf, dwg) — reference PDF 1.7, 1 page/sheet (F245)
```

### H.3.15 Document-format version and migration history
```jsonc
Migration { from: "1.3", to: "2.0", tool: "mkcad-migrate 2.0.1", at: ISO-8601, notes?: string, backup: "history/1.3.zip" }
```

## H.4 Sample contracts (brief §5 list 1–9)

### H.4.1 Complete project (abridged `document/document.json`)
```json
{
  "schema": "1.0",
  "id": "0b9f…",
  "name": "Bracket",
  "kind": "part",
  "units": { "length": "mm", "mass": "g", "angle": "deg" },
  "tolerances": { "modeling_mm": 1e-6, "sketch_mm": 1e-5, "angular_rad": 1e-9 },
  "rootComponent": "c-root",
  "timeline": { "items": ["s-1", "f-1", "f-2"], "marker": 3 },
  "namedViews": [{ "id": "v-top", "name": "TOP", "builtin": true,
                   "camera": { "kind": "orthographic", "eye": [0,0,100], "target": [0,0,0], "up": [0,1,0], "extent": 80 } }]
}
```

### H.4.2 Sketch and constraint system (`sketches/s-1.json`) — 40×20 rectangle, fully constrained (cf. F14)
```json
{
  "id": "s-1", "name": "Sketch1",
  "plane": { "kind": "plane", "label": { "feature": "origin", "role": "plane", "key": "XY" } },
  "points": [ { "id": "p:0", "x": 0, "y": 0, "fixed": true },
              { "id": "p:1", "x": 40, "y": 0 }, { "id": "p:2", "x": 40, "y": 20 }, { "id": "p:3", "x": 0, "y": 20 } ],
  "curves": [ { "id": "c:0", "kind": "line", "refs": { "start": "p:0", "end": "p:1" } },
              { "id": "c:1", "kind": "line", "refs": { "start": "p:1", "end": "p:2" } },
              { "id": "c:2", "kind": "line", "refs": { "start": "p:2", "end": "p:3" } },
              { "id": "c:3", "kind": "line", "refs": { "start": "p:3", "end": "p:0" } } ],
  "constraints": [ { "id": "k:0", "kind": "horizontal", "entities": ["c:0"] }, { "id": "k:1", "kind": "horizontal", "entities": ["c:2"] },
                   { "id": "k:2", "kind": "vertical", "entities": ["c:1"] },   { "id": "k:3", "kind": "vertical", "entities": ["c:3"] } ],
  "dimensions": [ { "id": "d:0", "kind": "horizontal", "entities": ["p:0", "p:1"], "driving": true, "expression": "Len", "placement": [20, -10] },
                  { "id": "d:1", "kind": "vertical", "entities": ["p:1", "p:2"], "driving": true, "expression": "20 mm", "placement": [50, 10] } ]
}
```

### H.4.3 Feature-creation request (command → domain)
```json
{
  "request": "feature.create", "requestId": "r-77", "documentId": "0b9f…", "insertAt": "marker",
  "feature": { "type": "extrude",
    "inputs": { "profiles": [ { "kind": "profile", "label": { "feature": "s-1", "role": "region", "key": "loop(c:0,c:1,c:2,c:3)" } } ] },
    "params": { "start": "profilePlane", "direction": "oneSide", "extent": "distance", "distance": "10 mm",
                "taperAngle": "0 deg", "operation": "newBody" } }
}
```
Defaults mirror the reference dialog [O] F69; `operation` default New Body, auto-switching per E.2.5 [O] F143–F145.
Validation errors return `FEATURE.PARAM.INVALID` (e.g. distance 0 — reference disables OK, F69).

### H.4.4 Feature-recompute result
```json
{
  "recomputeId": "rc-31", "trigger": { "kind": "edit", "node": "s-1" }, "durationMs": 42,
  "nodes": [
    { "id": "s-1", "health": "healthy", "solve": { "status": "solved", "dof": 0, "iterations": 4 } },
    { "id": "f-1", "health": "healthy", "outputs": { "bodies": 1, "faces": 6 },
      "references": [ { "input": "profiles[0]", "resolution": "exact" } ] },
    { "id": "f-2", "health": "error", "outputs": { "cached": true },
      "references": [ { "input": "edges[0]", "resolution": "lost" } ],
      "diagnostics": [ { "code": "REF.LOST", "severity": "error", "params": { "input": "edges[0]", "kind": "edge" } } ] }
  ],
  "rolledBack": ["f-3"]
}
```
`resolution` ∈ `exact | expanded(n) | signature | lost | ambiguous` (G.4 steps). Reference parallels: expanded F42,
lost F43, recovery F44.

### H.4.5 Geometry-kernel operation result (adapter → domain; kernel-neutral)
```json
{
  "op": "extrude", "kernel": { "id": "occt", "version": "7.9.0" }, "status": "ok",
  "bodies": [ { "label": { "feature": "f-1", "role": "body", "key": "0" }, "volume_mm3": 8000.0, "valid": true } ],
  "generated": [ { "label": { "feature": "f-1", "role": "side", "key": "c:0" }, "kind": "face" },
                 { "label": { "feature": "f-1", "role": "cap",  "key": "end" }, "kind": "face" } ],
  "modified": [], "deleted": [],
  "warnings": []
}
```
`status` ∈ `ok | failed | partial`; failures carry `KERNEL.*` codes (H.5). The adapter must produce labels from
authoritative inputs (G.4 step 1) — kernel enumeration indices never leave the adapter.

### H.4.6 Structured diagnostic
```json
{ "code": "SKETCH.CONSTRAINT.REDUNDANT", "severity": "error", "subject": { "sketch": "s-1", "entity": "d:2" },
  "params": { "suggestion": "make_driven" }, "learn": "help/sketch/over-constrained" }
```
Fields: `code` (stable, H.5), `severity` (`info|warning|error`), `subject` (IDs), `params` (typed per code), `learn`
(educational-layer link — reference offers a "Learn More" link in the over-constraint dialog [O] F38).

### H.4.7 Undoable command (wire form of H.3.11)
```json
{ "id": "u-210", "label": "Edit Sketch Dimension d:0", "kind": "sketch",
  "after":  [ { "op": "replace", "path": "/sketches/s-1/dimensions/0/expression", "value": "40 mm" },
              { "op": "replace", "path": "/sketches/s-1/points/1/x", "value": 40 } ],
  "before": [ { "op": "replace", "path": "/sketches/s-1/dimensions/0/expression", "value": "41.7509298375 mm" },
              { "op": "replace", "path": "/sketches/s-1/points/1/x", "value": 41.7509298375 } ] }
```
Values from the reference undo trace [O] F48/F51 (placement value stored at full precision, edit is a separate step).

### H.4.8 Import/export job
```json
{ "job": "export", "id": "j-9", "format": "step", "options": { "schema": "AP242", "units": "mm", "scope": "document" },
  "target": "C:/out/bracket.step", "state": "succeeded", "progress": 1.0, "cancellable": true,
  "result": { "bytes": 8921, "warnings": [] } }
```
`state` ∈ `queued | running | succeeded | failed | cancelled`. Reference exports STEP as AP214 [O] F46; PROPOSED
default AP242 with AP214 option (decision in J). Formats: STEP, STL (binary default, F47), 3MF, OBJ, DXF/SVG sketch;
IGES optional (legacy). Licence-gated formats in the reference (IGES/SAT, F45) are not a product model to copy.

### H.4.9 Plugin command contribution
```json
{ "plugin": { "id": "org.example.gearcalc", "version": "1.2.0", "api": "^1.0", "schema": "1.0",
              "capabilities": ["read-document", "create-features"], "entry": "python:gearcalc.main" },
  "contributes": { "commands": [ { "id": "gearcalc.spur", "title": "Spur Gear", "workspace": "design",
                                   "panel": "create", "icon": "icons/gear.svg", "shortcut": null } ] } }
```
Capabilities are an allow-list enforced by the host (failure containment, brief §6). `api` is a semver range checked at
load; incompatible plugins are disabled with `PLUGIN.API.INCOMPATIBLE`. Reference has a Scripts and Add-Ins manager
(Shift+S [O] F29) and run-on-startup add-ins [O] (harness MKProbe) — capability model is [P].

## H.5 Diagnostic code catalogue (stable; display text localised)

| Code | Severity | Raised by | Reference parallel |
|---|---|---|---|
| `SKETCH.CONSTRAINT.DUPLICATE` | error | add constraint | "Constraint has already been applied…" F60 |
| `SKETCH.CONSTRAINT.REDUNDANT` | error (+ suggestion make_driven) | add constraint/dimension | VCS_SKETCH_OVER_CONSTRAINTS F15; GUI offers driven F38 |
| `SKETCH.CONSTRAINT.CONFLICT` | error | add constraint | VCS_SKETCH_SOLVING_FAILED F11, F60 |
| `SKETCH.SOLVE.CONFLICT` | error | edit dimension | F11 |
| `SKETCH.SOLVE.NONCONVERGENT` | error | solver | [U] in reference |
| `SKETCH.DIM.INVALID_VALUE` | error | edit dimension (≤ 0 length etc.) | GUI rejects F39; API accepts F17/F18 (not adopted) |
| `SKETCH.MOVE.CONSTRAINED` | info | drag fully constrained point | API silently restores F19 |
| `SKETCH.OFFSET.EMPTY` | error | sketch offset | (artifact F) |
| `SKETCH.GEOMETRY.DUPLICATE` | warning | profile detection | (artifact F) |
| `SKETCH.REF.LOST` | warning | projected source / support gone | F27, F84 |
| `SKETCH.CONSTRAINT.REMOVED` | warning (params: constraint ids) | move/rotate that invalidates constraints | silent removal F137 |
| `REF.LOST` | error | feature input unresolved | F43 |
| `REF.AMBIGUOUS` | warning | split region, single-input feature | F116/F119 (reference silently picks one) |
| `GRAPH.REORDER.DEPENDENCY` | error | reorder | CIRCULAR_DEPENDENCY F24 |
| `GRAPH.DEPENDS_ON_FAILED` | warning | recompute | (G.3) |
| `FEATURE.PARAM.INVALID` | error | feature dialog | OK disabled at 0 F69 |
| `FEATURE.RESULT.TOPOLOGY_CHANGED` | warning | recompute | silent split F25 |
| `FEATURE.RESULT.NO_EFFECT` | warning | boolean/cut | silent no-op F26 |
| `FEATURE.RESULT.OVERLAP` | warning | pattern/mirror creating overlapping separate bodies | silent F173, F174 |
| `FEATURE.RESULT.CLAMPED` | warning | feature result shorter/smaller than requested (taper/draft apex) | silent F177, F179, F168 |
| `FEATURE.INPUT.DISJOINT` | warning | sweep profile not on its path | silent F180 |
| `PARAM.NAME.INVALID` | error | parameter add/rename | F54, F21 |
| `PARAM.EXPR.INVALID` | error | expression | F55 |
| `PARAM.EXPR.CYCLE` | error | expression | F55 |
| `PARAM.IN_USE` | error | delete parameter | API returns False silently F58 |
| `DRAWING.GENERATE.FAILED` | error | drawing generation job | reference silent F217/F236 |
| `IO.UNITS.ASSUMED` | info | import without declared/understood units | SVG 96-dpi F193 (S-10) |
| `DOC.RECOVERY.AVAILABLE` | info | start-up with recovery data | lost edits F215 (S-14) |
| `JOINT.LIMIT.CLAMPED` | warning | joint value beyond limit | F200 (S-15) |
| `KERNEL.OP.FAILED` | error (params: `reason` ∈ corner \| size \| offset \| other, kernel detail) | adapter | coded kernel messages with remedies F169 — keep the "try …" remedy text pattern [P] |
| `KERNEL.RESULT.INVALID` | error | adapter (invalid solid) | — |
| `IO.FORMAT.UNSUPPORTED` | error | import/export | IGES/SAT restricted F45 |
| `IO.FILE.CORRUPT` | error | open/import | — |
| `DOC.SCHEMA.NEWER` | warning | open | H.6 |
| `DOC.MIGRATION.FAILED` | error | open | H.6 |
| `PLUGIN.API.INCOMPATIBLE` | warning | plugin load | — |
| `JOB.CANCELLED` | info | jobs | — |
| `SKETCH.GEOMETRY.DEGENERATE` | warning (params: entity ids) | solver: solved but a corner is folded (e.g. collinear triangle) — PoC K-1, accepted 2026-10-14 | silent F106 |
| `SKETCH.INPUT.INVALID` | error | sketch command with a missing / wrong-kind reference, or editing a driven dimension — PoC K-1, accepted 2026-10-14 | — |

Rule: codes are append-only; a removed code stays reserved. Tests assert codes, never strings (artifact M).

## H.6 Versioning, migration and forward compatibility

| Entity | Validation | Migration | Forward compatibility | Corruption / partial recovery |
|---|---|---|---|---|
| manifest | schema, sha256 per part | rewritten on save | newer major → refuse with `DOC.SCHEMA.NEWER`, offer read-only open when minor-compatible | missing manifest → reconstruct from parts, warn |
| document.json | JSON-Schema + referential integrity (all IDs resolve) | major: migration step chain `n → n+1`, each idempotent and tested with fixture files | unknown fields kept verbatim (round-trip) | invalid → open last backup in `history/` |
| sketches/*.json | integrity: every constraint/dimension entity exists; points finite | per-major | unknown constraint kind → sketch opens read-only, constraint shown as "unsupported", geometry kept | one bad sketch → that sketch marked Error, others load |
| features/*.json | params parse; inputs are descriptors | per-major | unknown feature type → node kept, suppressed, `DOC.SCHEMA.NEWER` | bad feature → node Error, downstream DEPENDS_ON_FAILED |
| parameters.json | names unique; no cycles | per-major | unknown function → expression kept, value = cached | cycle on load → parameters involved Error |
| cache/* | kernel id + version + input hash | never migrated — dropped and rebuilt | ignored if unknown | ignored |

Ownership: document owns parameters, timeline and components; components own sketches/features (by id list);
sketches own their sub-entities. Dependencies are derived (H.3.3), never stored, to prevent drift.
Save always writes the current schema; opening an older file runs migrations into memory and records them in
`history/migrations.json` on next save (with backup of the original).

## H.7 Open questions
- H-Q1 Tolerance values (`modeling_mm`, `sketch_mm`) — set by PoC K-3/K-4.
- H-Q2 Persist solved coordinates for *fully* constrained sketches too (simpler) or only under-constrained? [P] all.
- H-Q3 Native camera/marker/selection persistence in the reference (N.1 X-16, needs one document slot).
