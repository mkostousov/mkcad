# MK Cad backlog (source for Jira project MKCAD)

Derived 2026-10-14 from `docs/PLAN-next.md`, `docs/spec/L-roadmap.md` (milestones → epics), `docs/spec/M-conformance-suite.md`
(→ acceptance criteria), `docs/spec/N-open-questions.md` §N.2a, `poc/KICKOFF.md`, `poc/RESULTS.md`.
Stack decided: Rust core, own solver (K-1 passed), OCCT 8 via vcpkg behind `IGeometryKernel` (C++ shim), wgpu renderer,
Python scripting API (PyO3), web edition later (wasm solver + remote kernel). Label `needs-user-review` = decision applied
with a recommendation, awaiting the user.

Status legend: DONE · NEXT · TODO

## Epics and tasks

| Epic | Task | Acceptance (M rows / criterion) | Status |
|---|---|---|---|
| K-1 Sketch solver PoC | Own Rust solver, fixtures, bench, determinism | SK-01…09,13,16…19; ≤50 ms; Win=Linux bits | DONE |
| K-1 Sketch solver PoC | Review PoC decisions (RESULTS.md) | accepted by the user 2026-10-14 | DONE |
| K-2 OCCT kernel + topological naming | K2-1 `occt-sys` crate: C++ shim + Rust bridge, links vcpkg OCCT, box volume test | box 10×20×30 volume = 6000 rel 1e-12 | DONE (poc/occt-kernel) |
| K-2 OCCT kernel + topological naming | K2-2 Kernel ops: extrude profile, boolean cut/fuse, fillet, with `BRepTools_History` exposed | ops return shape + history (generated/modified/deleted) | DONE (7 history tests) |
| K-2 OCCT kernel + topological naming | K2-3 Naming model per G.4: labels from sketch IDs + history, resolution exact/expanded/lost | unit tests per G.4 step | DONE (poc/occt-kernel/src/naming.rs, 12 tests; label model is pure logic, not yet wired to kernel history - wiring in K2-4; needs-user-review: string labels, fixed 1e-6 signature tol, edge label = first face label) |
| K-2 OCCT kernel + topological naming | K2-4 Replay REF-01…REF-08 | all REF rows resolve per G.4 | NEXT |
| K-2 OCCT kernel + topological naming | K2-5 STEP AP242 round-trip + fillet robustness smoke | IO-01 subset | TODO |
| K-3 Profile detection | K3-1 Planar arrangement over sketch curves (lines/arcs/circles), tol_lin 1e-5 mm | SK-10 1/0 profiles | TODO |
| K-3 Profile detection | K3-2 Region/loop extraction with holes, duplicate curves warning | SK-11 exact areas, SK-12 3 profiles | TODO |
| K-4 Determinism | K4-1 Document-level rebuild 100× Windows + Linux | bit-identical coords, volume rel 1e-12 (TX-DET) | TODO |
| M1 App shell + CI | Cargo workspace at repo root (solver, domain, kernel, app) | builds Win/Linux(/macOS) | TODO |
| M1 App shell + CI | GitHub Actions matrix windows/ubuntu/macos, clippy, wasm32 check | CI green | TODO |
| M1 App shell + CI | UI framework spike egui vs Slint vs Dioxus (web support is a criterion) | decision record; label needs-user-review | TODO |
| M1 App shell + CI | CommandBus skeleton + diagnostics sink (H.5 codes) | empty window, log viewer, cold start < 2 s | TODO |
| M2 Units + expressions + undo core | Quantity/units, expression evaluator ("," separator, N.2a-6), transactions | EX-01…EX-12, UN-01…UN-04 | TODO |
| M3 Viewport (wgpu) | wgpu renderer: grid, orbit/pan/zoom, view cube, ID-buffer picking, MSAA, high-DPI | VW-01…VW-06, 60 fps @ 1M tris | TODO |
| M4 Persistence | `.mkcad` package, autosave, recovery, migrations | PK-01…PK-08 | TODO |
| M5 Timeline + recompute | dependency graph, rollback/reorder/suppress | TX-03…TX-07, TX-DET | TODO |
| M6 Sketch tools | line/arc/circle/rectangle tools, trim/extend/break (N.2a-3), inference (N.2a-1) | SK-14, IN-01…IN-08 | TODO |
| M7 Solver integration | integrate K-1 solver as `ISketchSolver`, live DOF display | SK-01…09, SK-13 in app | TODO |
| M8 Diagnostics + learning layer v1 | over-constraint dialog (make driven), learner explanations | SK-03, SK-04, SK-06, SK-15; no silent outcome (S) | TODO |
| M9 Profiles in app | integrate K-3 | SK-10…SK-12 | TODO |
| M10 Extrude/revolve | kernel adapter ops, feature dialogs (E.2.5) | TX-01, TX-02, EXT-01…EXT-10; K-2 passed | TODO |
| M11 Booleans + downstream rebuild | history → labels | TX-03, TX-04, REF-01…REF-08 | TODO |
| M12 Features | fillet, chamfer, shell, draft, hole, patterns | FT-01…FT-20 | TODO |
| M13 Reference hardening | signature fallback | REF-*, K-4 met | TODO |
| M14 Exchange | STEP AP242, STL, 3MF | IO-01…IO-10 | TODO |
| M15 Post-MVP | assemblies/joints, drawings, sheet metal | JT, DR, SM rows | TODO |
| M16 Education + accessibility + deployment | learning layer v2, tutorials, screen reader, installers | AX-01…AX-08 | TODO |
| M17 Hardening + release | fuzzing, crash recovery, packaging, 1.0 | full M suite, fuzz 24 h clean | TODO |
| Python API | PyO3 module over CommandBus (N.2a-7: API = GUI) | scripted SK-01 identical to GUI | TODO |
| Web edition | solver + wgpu in browser, OCCT remote kernel service behind `IGeometryKernel` | sketch demo in browser | TODO |
| AI assistance | assistant acting only via undoable commands (ONNX/candle or cloud) | every AI action undoable | TODO |

## Jira sync

Jira project MKCAD does not exist on mikhako.atlassian.net (checked 2026-10-14); the Atlassian connector cannot create projects. Create it manually (key MKCAD, Jira Software), then sync this table.
