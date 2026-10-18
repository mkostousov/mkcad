# Reference study plan — assemblies and joints (X-19 … X-24)

Added 2026-10-17. Current evidence is thin: F4 (Assembly document kind), F126 (import creates an occurrence),
F141 (DOF Analysis exists), F199–F201 (one revolute joint, limit not clamped via API, transform override accepted).
Spec rows depending on it: M JT-01, G (no assembly model yet), E (no joint state machine), S (JOINT.LIMIT.CLAMPED only).

Method as before: Fusion API jobs (`vmharness/fusion/jobs/x19_*.py` …, results in `vmharness/results/`) first, GUI +
OCR only where the API cannot see the behaviour (dialogs, drag, warnings). Each finding becomes Fnnn in `docs/STUDY.md`
with an evidence class. VM rules: 3D acceleration OFF, 4 vCPU; all work inside ONE unsaved design unless noted
(10-document limit; X-24 needs saved documents).

| Batch | Topic | Experiments | Spec output |
|---|---|---|---|
| X-19 | Components and occurrences | new component (empty / from bodies); activate; copy-paste vs paste-new (shared definition vs copy); occurrence transform; ground / unground; Capture Position vs Revert after a move; rigid group; browser and timeline items for each | G.8 assembly model (component, occurrence, transform, ground); CD browser rows |
| X-20 | Joint types and DOF | rigid, revolute, slider, cylindrical, pin-slot, planar, ball: remaining DOF, motion values, flip, offset / angle, limits (min / max / rest); joint origins (face centre, edge mid, vertex, between two faces); as-built joint vs joint | H.3.19 joint contract; M JT-02…JT-10 |
| X-21 | Joint solving and conflicts | chain of three parts; closed loop (four-bar linkage); redundant / conflicting joints (two revolutes on skew axes); drag a jointed part in the GUI; limits in the GUI vs API (F200); motion of a grounded part; what warnings / errors appear and where | E.2.17 joint/assembly state machine; S rows; H.5 JOINT.* codes |
| X-22 | Motion and checks | motion link (gear ratio between two revolutes); contact sets; motion study; interference check; DOF Analysis tool (F141) | M JT rows; A.3 scope decision (in / later) |
| X-23 | Top-down references | sketch in component B projecting geometry of component A; edit A → B updates?; joint position in the timeline; suppress / delete a joint or a component and the effect on dependants | G.4 naming across components; REF rows for assemblies |
| X-24 | External components (needs user OK: 2 document slots) | insert a saved design into another; edit the source → "out of date" notice and update; break link; version pinning | PK rows for linked documents; N question for scope |

Order: X-19 → X-20 → X-21 → X-23 → X-22 → X-24. Estimated effort: one study session per two batches.
Jira: CAD-41 (X-19) … CAD-46 (X-24, needs-user-review) under epic CAD-19 (M15 Post-MVP), label `reference-study`.
