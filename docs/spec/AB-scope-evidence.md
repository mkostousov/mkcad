# A. Executive scope and non-goals · B. Evidence register and test environment (draft 1)

Status: draft, 2026-10-11. Tags: [O] [D] [I] [P] [U]. Findings Fnn in `docs/STUDY.md`.

## A.1 Product statement [P]

An educational, parametric, desktop-first mechanical CAD application for students, educators, makers and
entry-level engineers. It teaches transferable concepts — constrained sketches, degrees of freedom, feature history,
design intent, persistent references — with predictable, deterministic modelling behaviour and explanations a
learner can act on. It is specified from publicly observable behaviour of a reference application (Autodesk Fusion,
build 2705.1.30) plus public standards; it does not imitate the reference's look, branding or file format.

## A.2 Users and educational goals [P]

| Group | Needs that shape requirements |
|---|---|
| First-time learners | Progressive disclosure: MVP command set (CD D.3) with advanced options collapsed; plain-language diagnostics with `learn` links (H.4.6); visible DOF state (F.6); no silent outcomes (G.3: diagnose what the reference leaves silent, F25/F26/F116). |
| Classrooms / managed labs | Offline operation, per-machine install without account, deterministic rebuilds so an exercise file behaves identically on every seat (M TX-DET), file-based projects (H.1) that work on shared drives, policy-lockable preferences. |
| Self-directed makers | STL/3MF export for printing; STEP for exchange; quick primitives. |
| Progressing to professional CAD | Reference-like conventions where they are industry-wide (sketch → feature → timeline; Extrude E, Fillet F … shortcut letters as *defaults* [O] F29), standard terminology in tooltips. |
| Instructors | Templates, read-only "exercise" mode, parameter tables visible, rebuild diagnostics exportable as a report for grading (PROPOSED feature, later release). |
| Accessibility & localisation | Full keyboard operation of dialogs and command search (reference has `S` command search [O] F5), screen-reader names on every control (the reference's ribbon/tree/canvas are not exposed to UI Automation [O] F7 — a gap the new product must not repeat), configurable units, locale-independent expressions (N.2-6). |
| Learning mode (optional) | Explains constraints, DOF, feature dependencies and rebuild failures; never changes model semantics (brief §1.1). |

## A.3 Modelling scope by release [P]

| Area | MVP | First production release | Later professional | Out of scope |
|---|---|---|---|---|
| Documents | single-part document, save/open `.mkcad`, autosave + recovery | multi-document, templates | cloud/collaboration | proprietary reference formats (`.f3d`) |
| Sketch | line, arc (3-pt, center, tangent), circle, 2-pt/center rectangle, point, construction toggle, trim/extend, offset, fillet; constraints H/V, coincident, parallel, perpendicular, tangent, equal, midpoint, concentric, collinear, symmetric, fix; distance/radius/diameter/angle dims, driven dims; profile detection | ellipse, slot, polygon, spline (fit), mirror/pattern, project/include, text | conic, control-point splines, curvature (G2) constraint, 3D sketch | — |
| Features | extrude (new body, join, cut; distance / to-object / symmetric), revolve, fillet, chamfer, hole (simple), timeline edit/suppress/reorder/rollback | sweep, loft, shell, draft, patterns, mirror, thread (cosmetic) | rib, web, emboss, boundary fill, direct edit | T-spline/form modelling, generative design |
| Bodies/components | multi-body part | components & occurrences, basic joints | assemblies with motion | — |
| Construction | origin planes/axes, offset plane, plane at angle, midplane, axis through two points | full CONSTRUCT set (CD) | — | — |
| Inspect | measure, section view | interference, centre of mass | analyses (draft, curvature, zebra) | simulation |
| Materials/appearance | appearance colour, density for mass | material library | rendering | photoreal render |
| Exchange | STEP export (AP242), STL/3MF export, STEP import | OBJ, DXF/SVG sketch import/export, IGES import | IGES export | SAT/SMT proprietary kernels (reference restricts on Personal licence, F45) |
| Drawings, sheet metal, CAM, PCB, simulation | — | drawings: base/projected/section views, overall + chain dims, title block, PDF/DXF export (E.2.16, H.3.17) | sheet metal (K-factor flat pattern, SM-01…03), detail/break views, auto-dimension schemes | CAM, PCB, simulation, generative |
| Sketch observability | provenance + operation log stored (no UI) | sketch history panel + structure tree with per-entity DOF (reference has none, F48/F98/F224) | replayable sketch history (decide later) | — |
| Assisted constraining | — | learning-mode "suggest constraints" with explained DOF removal (reference AutoConstrain is paid, F230/F234) | — | — |

## A.4 Non-goals [P]

1. No pixel or brand imitation of the reference; layout follows the general CAD pattern (C.2), not its exact geometry.
2. No reproduction of reference behaviours classified as defects or non-determinism: silent body splits (F25),
   silent no-op cuts (F26), non-reproducible split-region choice (F116/F119), API/GUI divergence (F9 vs F36,
   F15 vs F38, F17 vs F39), silent zero-valued expressions (F56).
3. No cloud dependency for core modelling; no account required to open, edit or save.
4. No licence-tier feature gating of file formats as a product model (F45 is an observation only).
5. Not a kernel research project: the geometry kernel is adopted (J), isolated behind an adapter.

## A.5 Standards baseline [P]

| Standard | Use | Mandatory for | Conformance statement required (brief §1.3) |
|---|---|---|---|
| ISO 10303-21 (STEP physical file) | exchange container | MVP | edition, test files, known limits |
| ISO 10303-242 (AP242 ed. 2+) | default STEP schema (B-rep geometry, PMI later) | MVP export, FPR import | AP242 conformance class for geometry only; validation via third-party viewer/checker |
| ISO 10303-214 (AP214) | compatibility option (reference default [O] F46) | FPR | — |
| IGES 5.3 | legacy import | FPR (import only) | — |
| STL (binary + ASCII), 3MF Core Spec 1.x, OBJ, glTF 2.0 | mesh exchange / print | MVP (STL, 3MF), FPR (OBJ, glTF) | 3MF consortium validator |
| DXF (AutoCAD R2018 subset), SVG 1.1 | 2D sketch exchange | FPR | entity subset list |
| ISO 80000-1/-3 (quantities & units) | unit naming/symbols, SI + imperial | MVP | — |
| ASME Y14.5 / ISO 1101 (GD&T) | concepts for teaching; PMI later | Later | — |
| WCAG 2.2 AA (adapted for desktop), platform accessibility APIs (UIA, NSAccessibility, AT-SPI) | accessibility | MVP for dialogs/command search; FPR for tree/canvas alternatives | audit checklist |
| SemVer 2.0 | app, plugin API, document schema (H.0) | MVP | — |

The product must not claim compliance without stating edition, conformance class, test procedure and known
limitations (brief §1.3).

## B.1 Test environment (reference)

| Item | Value | Source |
|---|---|---|
| Reference application | Autodesk Fusion, "Personal (Not for Commercial Use)" | window title |
| Build | 2705.1.30 | API `app.version` (results/env.json) |
| OS | Windows 10 22H2, build 19045.5854 (en-US), VirtualBox 7.1.4 guest, Guest Additions 7.1.4 | `ver`, driver versions |
| Host virtualisation | Hyper-V/VBS active on host → VirtualBox NEM mode | VBox.log |
| Graphics | VirtualBox WDDM 7.1.4, 3D on, 256 MB VRAM (Fusion reports 2304 MB dedicated) | Fusion AppLog |
| CPU / RAM | 4 vCPU (8 vCPU caused guest BSOD 0x0A), 16 GB | VM config, STUDY stability log |
| Display | 1920×1080, 100 % scaling | harness |
| Units / locale | mm default; en-US; expression separator ";" (F53) | env.json, F53 |
| Documents | only unsaved "Untitled" documents used (Personal: 10 editable documents) | protocol |

**Reliability impact of the environment (brief: state omissions):** single OS (no macOS) → keyboard shortcuts and
platform behaviours are Windows-only [O]; macOS rows in the command inventory stay [U]. Virtualised GPU → visual
artefacts (dimension text not drawn, F40) are not attributed to the reference. One build only → behaviour may
change between Fusion releases; every [O] row is tied to 2705.1.30. Personal licence → some commands/formats are
gated (F30, F45); their absence is not a reference design statement.

## B.2 Evidence methods and confidence

| Method | Evidence produced | Confidence | Limits |
|---|---|---|---|
| Public scripting API (MKProbe add-in, `fjob.ps1` jobs) | JSON results in `vmharness/results/` | High (direct state readout) | API may differ from GUI (F9/F15/F17); API ≠ internals — never stated as internals |
| GUI automation (VirtualBox COM mouse/keyboard) + annotated step screenshots | `vmharness/sessions/<run>/` (steps.jsonl, PNG, report.md) | High for what is shown | timing/focus loss risk (STUDY stability log) |
| OCR (Windows OCR) | text in `inventory/menus/*/*.txt`, console logs | Medium (mis-reads, single letters dropped) | shortcut letters verified from images |
| UI Automation tree | `results/uia_dump.txt` | High for layout regions | ribbon/tree/canvas opaque (F7) |
| Exported files | `C:\mkprobe\exports\*` headers | High | — |
| Public documentation | cited per row as [D] | Medium–High | version drift |

Confidence scale used in other artifacts: High = reproduced ≥ 2× or direct readout; Medium = single observation
or OCR; Low = inferred.

## B.3 Evidence register (index)

The full register is `docs/STUDY.md` (findings F1…F12x, each with class and evidence path) — kept as the single
source to avoid divergence. Grouped index:

| Topic | Findings | Artifact using them |
|---|---|---|
| Environment, workspaces, command inventory, shortcuts | F1–F8, F29–F35, F101–F104, F110 | CD |
| Sketch solver, constraints, dimensions, inference, profiles, edit ops | F9–F21, F36–F40, F60–F67, F73–F100, F105–F109 | F |
| Timeline, rebuild, suppress, reorder, delete, references | F22–F28, F41–F44, F111–F124 | E, G |
| Undo/redo | F48–F51, F111, F112 | E, H |
| Parameters & expressions | F52–F59 | E, H |
| Export | F45–F47 | H, J |
| Dialogs & transactions | F68–F72, F113 | E |

Reproduction: every job script is in `vmharness/fusion/jobs/`; `docs/RESUME.md` describes the lab bring-up.
