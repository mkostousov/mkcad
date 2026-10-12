# G. Domain model and dependency graph (draft 1)

Status: draft, 2026-10-11. Tags: [O] [D] [I] [P] [U]; evidence in `docs/STUDY.md`. This artifact is mostly
PROPOSED; reference behaviour is cited only where it constrains the design.

## G.1 Entity hierarchy [P]

```text
Project
└── Document (schema version, units, design kind)              ref: kinds Part/Assembly/Hybrid F4, F104
    ├── Settings (units, default tolerances, parameter table)
    ├── Parameter[]  user + model parameters, one name space    ref: shared dN counter F59
    ├── Component (root)
    │   ├── Origin (point, X/Y/Z axes, XY/XZ/YZ planes; hidden by default)   ref: F104
    │   ├── ConstructionGeometry[] (planes, axes, points)
    │   ├── Sketch[]  → owned points, curves, constraints, dimensions (artifact F)
    │   ├── Body[]    (result of features; derived, cached)
    │   ├── Occurrence[] → Component (FPR) — instances share one definition: editing the
│   │   component changes all occurrences [O] F190; "independent copy" duplicates the definition [O] F191;
│   │   reference keeps ONE design-level timeline including component features [O] F189
    │   └── Appearance / Material assignments
    ├── Timeline: ordered FeatureNode[] + marker index         ref: F22, F111
    └── NamedView[] (TOP, FRONT, RIGHT, HOME)                   ref: F104
```

Authoritative (persisted) vs derived [P]: parameters, sketches (geometry + constraints + dimensions), feature
definitions, timeline order, suppression, marker, references (as *reference descriptors*, G.4) are authoritative.
B-rep bodies, tessellation, profiles, solved coordinates of under-constrained sketches *may* be cached, but the
document must rebuild to the same result without caches (determinism requirement, artifact M).

Open point: solved sketch coordinates are part of the authoritative state, because under-constrained solutions are
path-dependent (F10, F108). Storing only constraints would not reproduce the user's sketch. **[P]** Persist
coordinates as authoritative; solver uses them as the initial guess.

## G.2 Feature node [P]

```text
FeatureNode {
  id: StableId            // never reused
  type: "extrude" | "revolve" | "fillet" | "sketch" | ...
  params: { name → Expression }     // expressions keep units (F20)
  inputs: ReferenceDescriptor[]     // profiles, faces, edges, planes
  suppressed: bool                  // F23
  health: Healthy | Warning | Error | Suppressed | RolledBack   // F22, F23, F27, F43
  diagnostics: Diagnostic[]         // stable codes (artifact H)
  outputs: { bodies, faces, edges } // derived, with generated-topology labels (G.4)
}
```

## G.3 Dependency graph and recompute [P]

- Nodes: parameters, sketches, features, construction geometry. Edges: "reads parameter", "references topology
  produced by", "sketch on face of", "projects from".
- The timeline is a topological order of this graph that the user may change within dependency limits. Reorder
  that violates an edge is rejected (`GRAPH.REORDER.DEPENDENCY`; reference rejects with CIRCULAR_DEPENDENCY [O] F24).
- Recompute after an edit = all nodes downstream of the edited node in timeline order up to the marker [O] F12, F22.
  Nodes after the marker are RolledBack and not evaluated.
- Parameter cycles are rejected at assignment time (`PARAM.EXPR.CYCLE`; reference [O] F55).
- Recompute of one node never mutates authoritative data of another node, except re-resolved reference descriptors
  (G.4), which are written back only on user commit [P].
- Partial rebuild [P]: a failing node gets Error + diagnostics + cached outputs (reference keeps cached geometry,
  F43, F84); downstream nodes evaluate against the cached outputs and are marked Warning `DEPENDS_ON_FAILED`.
- Surprising silent outcomes in the reference that the new product must diagnose [O]→[P]: body count change after a
  rebuild (F25) → `FEATURE.RESULT.TOPOLOGY_CHANGED`; boolean with no effect (F26) → `FEATURE.RESULT.NO_EFFECT`.

## G.4 Persistent references (topological naming) [P]

Reference behaviour that the scheme must meet or improve [O]:
| Case | Reference | Ref |
|---|---|---|
| Unrelated upstream feature inserted | fillet keeps its edge | F41 |
| Edge split into two by upstream groove | fillet applied to both pieces, Healthy | F42 |
| Edge removed upstream | Error "Edge 1 missing … using cached geometry" | F43 |
| Cause removed | automatic recovery, original geometry | F44 |
| Sketch on face, face resized | sketch follows | F28 |
| Face deleted | sketch kept with cached plane, Warning | F27 |
| Projected edge source deleted | curve kept, Warning | F84 |
| Support face split by upstream groove | sketch Healthy; profile built from the face region re-resolves to one piece only, silently (part of the cut lost); piece rule unknown | F116, F117 |
| Shell's removed face split upstream | both pieces removed, Healthy | F118 |

Proposed scheme:
1. **Generated-topology labels.** Every kernel operation result is post-processed by the feature adapter into labels
   of the form `(featureId, role, index-in-role)` — e.g. extrude: `side(profileCurveId)`, `cap(start|end)`,
   `edge(side a ∩ cap end)`. Labels derive from *authoritative* inputs (sketch curve IDs), never from kernel
   enumeration order.
2. **Modified/split/merged history.** Booleans, fillets and splits report for each output face/edge the input labels
   it descends from (kernel history API — OCCT `BRepTools_History`-style; to be validated in PoC K-2).
3. **Reference descriptor** stored in a feature input: `{label path, geometric signature (type, centroid, normal,
   length/area), selection-time context}`.
4. **Resolution** at recompute: exact label match → done; label split into n → reference set expands to all
   descendants (matches F42); label vanished → try geometric signature within tolerance; else `REF.LOST` with cached
   geometry (matches F43) and automatic retry on later recomputes (matches F44).
   Split faces/regions behave like split edges (all descendants), not like the reference's single-piece pick
   (F116); if the feature semantics cannot take several pieces, emit Warning `REF.AMBIGUOUS` instead of choosing.
5. Resolution result is reported per reference in the recompute result (artifact H contract #4) so tests can
   assert it.

Risk: this is one of the two highest-risk areas (with the solver). Roadmap must run it as an early spike
(artifact L milestone 13 pulled forward as a PoC alongside milestone 10).

## G.5 Selection model [P]

- Selection references use the same descriptors as G.4, so a selection survives recompute.
- Selection is not undoable and not persisted [O] F72 (OK/Cancel clear selection; not an undo step).
- Preselection precedence (vertex > edge > face > body > component) [P]; reference click priority [O] F147/F148:
  vertex wins within ≈ 8 px (edge at 12 px), edge wins within ≈ 6 px (face at 8 px). Bands are screen-space, so
  zoom-independent in pixels [P]; learners may get larger bands (cf. N.2-1).

## G.6 Units [P]
- One internal length unit per document schema version; display unit is a document setting; expressions keep
  their written units (F20); unitless numbers in length fields = display unit (F71); angles stored in radians
  (F52) and displayed in degrees.
- Changing the display unit never rewrites stored expressions.

## G.7a Later-phase domain notes (from reference observations)

| Area | Reference [O] | Proposed model [P] |
|---|---|---|
| Sheet metal | Rule = named set {thickness, K-factor, bend radius as expression of thickness} chosen per body; library + "in this design" copies (F202); base/edge flanges are timeline features (F203, F204); flat pattern is a separate environment and NOT a timeline item, auto-updated (F205, F206) | `SheetMetalRule` document entity (H.3.16); flat pattern = derived view of a sheet-metal body (cache), regenerated on recompute; unfold length = K-factor allowance (M SM-03) |
| Joints | Grounding ("Pin") and joints are timeline items (F199); joint values drivable; limits; direct transform of a jointed occurrence not enforced via API (F201) | Joint = feature node between two occurrence references; solved by an assembly solver port (later phase); direct transforms of jointed parts are rejected or re-solved [P] |
| Components | one design-level timeline (F189); shared definition across occurrences (F190) | as G.1 |

## G.7 Open questions
- G-Q1 Reference behaviour when a split edge is later re-merged (reference set shrinks?).
- G-Q2 Partly answered (F116, profile from a split face → one piece). Still open: rule for which piece; face
  references of fillet/shell/draft on split faces.
- G-Q3 Selection priority under the cursor in the reference.
