# L. Implementation roadmap (draft 1)

Status: draft, 2026-10-11. All **[P]**. Vertical, testable milestones (brief §8). Test IDs → artifact M; ports → I.3;
risks → K. The solver (K-1) and persistent references (K-2) are pulled forward as spikes — they run in parallel with
M3–M5, not at the end (brief §8).

| M | User-visible capability | Components introduced | Interfaces stabilised | Dependencies selected | Required tests | Performance target | Failure & recovery criteria | Definition of done | Risks / exit criteria |
|---|---|---|---|---|---|---|---|---|---|
| 0 | none — study protocol, scope, glossary | docs (A–N) | terminology, evidence classes | — | review checklist | — | — | artifacts A–N reviewed; N.2 decisions taken | exit: N.2 decisions 1–7 recorded |
| 1 | app starts, empty window, log viewer | app shell, CommandBus skeleton, diagnostics sink, CI | `IDiagnosticsSink`, diagnostic code format (H.5) | CMake, vcpkg, Qt 6, Catch2 | build on 3 OSes; clang-tidy clean | cold start < 2 s | crash → minidump written | CI green on Win/macOS/Linux | toolchain pinning works (K-R11) |
| 2 | units shown in a parameter table; expressions evaluate | Quantity/units, expression evaluator, StableId, Transaction/Undo core | `IExpressionEvaluator`, H.3.6, H.3.11 | Eigen, (mp-units concepts) | EX-01…EX-12 (expressions), UN-01…UN-04; fuzz parser | 10k expressions evaluate < 50 ms | invalid input → PARAM.* codes, no state change | property tests (associativity/units) pass | grammar decision N.2-6 made |
| 3 | 3D viewport with grid, orbit/pan/zoom, view cube, selection of a dummy body | renderer port + OCCT AIS impl, camera, picking, SelectionService | `IRenderer`, H.3.10, G.5 | OCCT (visualization only) | VW-01…VW-06 | 60 fps at 1M triangles on reference HW | GPU init failure → software fallback with banner | navigation conventions match C.1/F6 (wheel, MMB pan, Shift+MMB orbit) | **disposable geometry spike: OCCT box/fillet/STEP round-trip** |
| 4 | save/open `.mkcad`, autosave, recover after kill | package repository, migration framework, autosave job | `IDocumentRepository`, H.1, H.6 | minizip-ng | PK-01…PK-08, fixture round-trips, fuzz package reader | open 10 MB package < 1 s | corrupted part → partial open per H.6 | kill -9 during save never corrupts last good file | storage variant final (K: reversible until here) |
| 5 | timeline with dummy features; edit/suppress/reorder/rollback | dependency graph, recompute engine, Timeline UI | G.3, H.3.3, H.4.4 | — | TX-03…TX-07 (with stub features), TX-DET | recompute 200 stub nodes < 20 ms | node failure → Error + cached outputs, downstream DEPENDS_ON_FAILED | rollback/reorder semantics match E.2.7–E.2.9 | — |
| 6 | sketch on plane: draw lines/arcs/circles, trim, construction | sketch model, tools (State pattern), inference | H.3.4, F.2, F.4 | — | SK-14, inference IN-01…IN-08 | draw at 60 fps with 2k entities | invalid geometry rejected with SKETCH.* | tools follow E.2.3/E.2.4 | inference thresholds N.2-1 |
| 7 | constraints & dimensions solve live (PoC → integrated) | solver adapter (PlaneGCS), DOF display | `ISketchSolver`, F.5/F.6 | PlaneGCS (K-R3) | SK-01…SK-09, SK-13 | **K-1: ≤ 50 ms / 500 entities** | conflict → previous solution kept (SK-05) | K-1 exit criterion met | **exit if K-1 fails → libslvs** |
| 8 | production sketch editing: driven dims, over-constraint dialog, diagnostics in learner language | diagnostics UI, learning layer v1 | H.4.6, F.6 | — | SK-03, SK-04, SK-06, SK-15 | — | every solver failure has a code + explanation | no silent outcome in SK suite | — |
| 9 | profiles highlight; closed regions selectable | planar arrangement, profile detection | F.8 | — | SK-10…SK-12 (K-3) | 1k-curve sketch regions < 100 ms | duplicate geometry → SKETCH.GEOMETRY.DUPLICATE | K-3 exit met | tolerance values H-Q1 fixed |
| 10 | extrude & revolve (new body/join/cut), dialog per E.2.5 | kernel adapter ops, labels from sketch IDs | `IGeometryKernel`, H.4.5 | OCCT modelling | TX-01, TX-02, EXT-01…EXT-10 | extrude of 100-curve profile < 200 ms | kernel failure → KERNEL.OP.FAILED, model unchanged | **K-2 spike passes** (labels stable across edits) | **kernel/naming irreversible after this milestone** |
| 11 | booleans, edit earlier feature, downstream rebuild | history → labels (G.4 steps 2–4) | `ITopologyNamer` | — | TX-03, TX-04, REF-01…REF-08 | rebuild 50-feature part < 1 s | REF.LOST / REF.AMBIGUOUS emitted per G.4 | reference cases F41–F44, F116, F118 all pass | — |
| 12 | fillet, chamfer, shell, draft, hole, patterns | feature implementations | FeatureNode params per type | — | FT-01…FT-20 | fillet 12 edges < 500 ms | fillet failure → FEATURE.* code, cached outputs | each feature has dialog + undo + suite tests | OCCT fillet robustness (K-R1) |
| 13 | references survive realistic edits (hardening) | signature fallback, retry on recompute | G.4 step 4 | — | REF-*, TX-DET 100× on 2 OSes (K-4) | — | — | K-4 met | — |
| 14 | STEP AP242 export/import, STL, 3MF | translators, job UI | `ITranslator`, H.4.8 | lib3mf, OCCT STEP | IO-01…IO-10 incl. third-party validators | 50 MB STEP import < 30 s, cancellable | import errors → IO.* codes, partial import report | conformance statement written (A.5) | — |
| 15 | (post-MVP) components/assemblies, drawings, sheet metal | per workstream | — | — | per workstream | — | — | — | scope per A.3 |
| 16 | educational mode, tutorials, accessibility, classroom deployment | learning layer v2, MSI/PKG/AppImage, policy prefs | — | — | AX-01…AX-08 (screen reader), deploy tests | — | — | WCAG-derived checklist passes | — |
| 17 | hardening & release | fuzzing at scale, crash recovery, packaging | all public contracts frozen for 1.0 | — | full M suite, fuzz 24 h clean | all targets above | crash recovery restores last autosave | release checklist | — |

## L.1 Spikes in parallel

| Spike | Start | Must finish before | Exit criterion (N.3) |
|---|---|---|---|
| K-1 solver | M2 | M7 | SK-01…SK-13 pass; ≤ 50 ms / 500 entities |
| K-2 naming | M3 | M10 | F41–F44, F116 cases resolve per G.4 |
| K-3 profiles | M5 | M9 | SK-10…SK-12 exact |
| K-4 determinism | M5 | M13 | 100 rebuilds × 2 OS bit-identical coords, volume rel 1e-12 |
