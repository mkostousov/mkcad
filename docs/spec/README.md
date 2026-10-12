# Clean-room CAD specification — index (draft 1, 2026-10-11)

Brief: `D:/Users/mikhail/SynologyDrive/Work/AI/clean-room-cad-fsd-prompt.md`. Evidence: `docs/STUDY.md` (findings Fnn).
Reference: Autodesk Fusion 2705.1.30, Windows 10 (see B.1 for reliability limits). Tags: [O] [D] [I] [P] [U].

| Artifact | File |
|---|---|
| A. Executive scope and non-goals · B. Evidence register and test environment | `AB-scope-evidence.md` |
| C. Workspace and component hierarchy · D. Command inventory | `CD-workspace-commands.md` |
| E. State-machine catalogue | `E-state-machines.md` |
| F. Sketch and constraint-system specification | `F-sketch-solver.md` |
| G. Domain model and dependency graph | `G-domain-model.md` |
| H. Data contracts and versioning rules | `H-data-contracts.md` |
| I. Architecture · J. Technology decision matrix · K. Dependency-risk register | `IJK-architecture-tech-risk.md` |
| L. Implementation roadmap | `L-roadmap.md` |
| M. Black-box conformance test suite | `M-conformance-suite.md` |
| N. Open questions and required experiments | `N-open-questions.md` |
| S. Silent-outcome requirements (draft 3 addendum) | `S-silent-outcomes.md` |
| F.11 Own 2D solver design (PoC K-1) | `F11-solver-design.md` |
| Change log | `CHANGELOG.md` |

## 1. Decisions that must be made before implementation (prioritised)

1. **API and GUI behave identically** (reference diverges: F9/F36, F15/F38, F17/F39) — recommended: one command layer for both (N.2-7).
2. **Persistent-reference policy for split regions/faces:** expand to all pieces vs `REF.AMBIGUOUS` — never a silent pick (N.2-4, G.4; reference F116/F119).
3. **Solved sketch coordinates are authoritative state** (G.1, H-Q2; reference path dependence F10).
4. **Geometry kernel = OCCT behind `IGeometryKernel`**, conditional on PoC K-2 (J.1). Irreversible after roadmap M10.
5. **Solver = PlaneGCS behind `ISketchSolver`**, conditional on PoC K-1; licence review of fallback libslvs (J.2).
6. **Expression grammar** locale-independent, argument separator "," (reference ";" F53), reserved unit names, no silent constants (N.2-6, H.3.6).
7. **Inference thresholds** for learners (reference 8 px / ~9 px / 2 px, F85–F93) (N.2-1).
8. **Line-symmetry semantics** (infinite-line mirror like the reference F65, or endpoint mirror) (N.2-2).
9. **Native package format** = hybrid ZIP with authoritative/cache split (H.1).
10. **Default STEP schema** AP242 with AP214 option (A.5, H.4.8; reference AP214 F46).
11. Break vs Trim collinearity rule (N.2-3).
12. Diagnose silent reference outcomes with stable codes (H.5) — confirm as product requirement. Catalogue of silent
    outcomes observed: body split by rebuild (F25), cut with no effect (F26), oversize fillet roll-over (F168),
    taper/draft clamped at apex (F177, F179), overlapping pattern/mirror bodies (F173, F174), sweep profile off its
    path (F180), angle dimension collapsing a line (F134), sketch rotate/scale deleting constraints/dimensions
    (F137, F207), split-region single-piece pick (F116/F119), SVG unit misread (F193), zero/negative dims via API (F17/F18).
13. Extrude/feature operation auto-selection (New Body → Cut/Join from preview) — adopt with explicit user override (F143–F145).
14. Sketch shortcuts: adopt verified reference letters L R C D T O P M (F138); Ctrl-held suppresses inference (F164).

## 2. Proof-of-concept experiments before committing to the architecture (prioritised)

| # | PoC | Exit criterion | Blocks |
|---|---|---|---|
| 1 | **K-2 topological naming** on OCCT history: extrude + boolean + fillet labels; replay REF-01…REF-08 | all REF cases resolve per G.4 | roadmap M10 (kernel lock-in) |
| 2 | **K-1 solver**: PlaneGCS behind port; SK-01…SK-13, SK-16…SK-18 | all pass; ≤ 50 ms / 500 entities; conflict keeps previous solution | M7 |
| 3 | **K-4 determinism**: 100 rebuilds × Windows + Linux | bit-identical solved coords; volume rel 1e-12 | M13, classroom requirement |
| 4 | **K-3 profile detection** with tol 1e-5 mm | SK-10…SK-12 exact | M9 |
| 5 | OCCT fillet robustness suite (FT-*) incl. oversize radius (F25) | failures produce codes, never invalid solids | M12 |
| 6 | Accessibility spike: Qt 6 custom ribbon + tree + timeline with a screen reader | AX-01…AX-04 pass | M16 (but UI tech reversible only until M3) |
| 7 | STEP AP242 round-trip through two independent checkers | IO-01 passes | M14 |
