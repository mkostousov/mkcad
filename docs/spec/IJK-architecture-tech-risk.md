# I. Architecture · J. Technology decision matrix · K. Dependency-risk register (draft 1)

Status: draft, 2026-10-11. All content **[P]** (design), grounded in reference observations where cited (Fnn,
`docs/STUDY.md`). Contracts referenced: artifact H. Behaviour referenced: E, F, G.

## I.1 Style

Modular monolith, ports-and-adapters. The domain core (document model, dependency graph, transactions) has no
dependency on UI toolkit, geometry kernel, solver, storage or scripting runtime. Constructor injection through
explicit interfaces; no DI container in the core (composition root in the app shell wires ~20 services by hand —
add a container only if wiring exceeds that).

## I.2 Component diagram

```text
                         ┌──────────────────────── Presentation (Qt 6) ─────────────────────────┐
                         │ Ribbon/CommandSearch · Browser · Timeline · Dialogs · Viewport widget │
                         │ Learning layer (explanations, DOF panel) · Accessibility bridge        │
                         └───────────────┬──────────────────────────────┬────────────────────────┘
                                         │ Command requests (H.4.3)     │ Events (doc changed, recompute result)
                         ┌───────────────▼──────────────────────────────┴────────────────────────┐
                         │ Application layer: CommandBus · ToolStateMachines (E) · UndoService    │
                         │ SelectionService (G.5) · JobScheduler · PluginHost (capabilities)      │
                         └───────┬──────────────┬───────────────┬──────────────┬─────────────────┘
                                 │              │               │              │
   ┌─────────────────────────────▼──┐  ┌────────▼────────┐ ┌────▼──────────┐ ┌─▼────────────────────┐
   │ Domain core                     │  │ Recompute engine│ │ Persistence   │ │ Interchange          │
   │ Document · Parameters · Sketch  │  │ (dependency     │ │ (package H.1, │ │ (STEP/STL/3MF/OBJ/   │
   │ model · FeatureNode · Timeline  │  │ graph, G.3)     │ │ migrations    │ │ DXF/SVG jobs H.4.8)  │
   │ ReferenceDescriptor (G.4)       │  │                 │ │ H.6)          │ │                      │
   └──────┬────────────────┬────────┘  └──┬───────────┬──┘ └───────────────┘ └──────────┬───────────┘
          │ SolverPort      │ ExprPort      │ KernelPort │                                 │ KernelPort
   ┌──────▼──────┐  ┌───────▼──────┐  ┌────▼───────────▼──────┐  ┌──────────────────┐     │
   │ Solver      │  │ Expression   │  │ Kernel adapter (OCCT): │  │ Tessellator /    │◄────┘
   │ adapter     │  │ evaluator    │  │ ops + history → labels │  │ Renderer port    │
   │ (PlaneGCS)  │  │ (units)      │  │ (G.4 steps 1–2)        │  │ (OCCT AIS → own) │
   └─────────────┘  └──────────────┘  └────────────────────────┘  └──────────────────┘
```

## I.3 Ports (interfaces) and owners

| Port | Methods (abridged) | Contract | Replaceable by |
|---|---|---|---|
| `ISketchSolver` | `solve(SketchSnapshot, SolveOptions) → SolveResult{status, coords, dof, conflicts[], redundant[]}` | F.5/F.6, H.4.4 `solve` | libslvs, own Newton/LM |
| `IGeometryKernel` | `extrude/revolve/boolean/fillet/... (inputs) → KernelResult` (H.4.5) | H.4.5 | other B-rep kernel |
| `ITopologyNamer` (inside kernel adapter) | `label(result, featureInputs) → GeneratedLabel[]`; `history(op) → modified/split/deleted` | G.4 | — |
| `IExpressionEvaluator` | `evaluate(expr, scope) → Quantity` ; `dependencies(expr)` | H.3.6 | — |
| `IDocumentRepository` | `open(path) / save(doc, path) / migrate(...)` | H.1, H.6 | SQLite variant |
| `ITranslator` | `import(path, opts) / export(doc, opts)` per format | H.4.8 | per format |
| `IRenderer` | `upload(meshes)`, `pick(x,y) → PickResult`, `setCamera` | H.3.10 | Vulkan/own GL |
| `IPlugin` | `activate(HostApi, capabilities)` | H.4.9 | — |
| `IDiagnosticsSink` | `report(Diagnostic)` | H.4.6 | telemetry (opt-in) |

## I.4 Patterns

| Pattern | Where | Note |
|---|---|---|
| Command | every user action → `UndoEntry` (H.3.11) | granularity rules from F48–F51, F111, F112 |
| Memento / operation log | Undo stores JSON patches over authoritative parts only | caches excluded |
| State | interactive tools (sketch line, dimension, extrude dialog) | E.2 state machines; in-dialog undo separate (F112) |
| Strategy | solver backend, kernel backend, inference thresholds (N.2-1) | — |
| Adapter | kernel, solver, translators, renderer | no kernel type leaks past adapter |
| Observer | document → UI events; recompute progress | events carry IDs, not objects |
| Factory | FeatureNode creation from type id; plugin command contributions | — |
| Repository | package persistence | H.1 |
| Dependency-graph evaluation | recompute engine | G.3; deterministic order = timeline order |

## I.5 Threads, cancellation, ownership [P]

| Work | Thread | Rule |
|---|---|---|
| Document mutation, undo, selection | UI/main thread only (single writer) | all mutations via CommandBus; recompute reads an immutable snapshot |
| Recompute (solver + kernel) | one recompute worker | works on snapshot; result applied on main thread atomically; a new edit cancels the running recompute (cooperative token checked between nodes and inside long kernel ops where the kernel supports it) |
| Tessellation | pool (N-1 cores) | per-body jobs, cancellable, results keyed by (body label, recompute id) |
| Rendering | render thread | consumes immutable mesh buffers |
| File I/O, import/export | job pool | progress + cancel (H.4.8); never touches live document — export uses snapshot |
| Plugins | main thread via HostApi (MVP); later out-of-process | capability check per call |

Determinism: recompute order and floating-point paths must not depend on thread scheduling (parallel tessellation
only after solids are final). PoC K-4.

## I.6 Engineering quality model

| Area | Requirement |
|---|---|
| Builds | CMake presets; vcpkg manifest with pinned baseline; reproducible builds on Windows/macOS/Linux CI |
| CI | per-PR: build 3 OSes, unit + integration tests, conformance suite M (headless), clang-tidy, clang-format check |
| Static analysis | clang-tidy (bugprone, performance, cppcoreguidelines subset), sanitizers (ASan/UBSan) nightly |
| Tests | unit (Catch2), property-based (rapidcheck) for solver and expression evaluator, regression fixtures (H.1 packages), fuzzing (libFuzzer) for package reader, STEP/STL/3MF importers, expression parser |
| Performance | benchmark suite in CI with thresholds (L milestones); solve ≤ 50 ms / 500 entities (K-1) |
| Diagnostics | structured logs (JSON), diagnostic codes (H.5), crash reporter (minidump, opt-in upload) |
| Compatibility | document fixtures from every released schema version must open (H.6); plugin API semver |
| Extensions | plugin failures isolated: exceptions caught at HostApi boundary, plugin disabled, user informed |

## J. Technology decision matrix

Scores 1–5; weights in brackets. Totals = Σ weight × score.

### J.1 Geometry kernel

| Criterion (w) | Open CASCADE (OCCT 7.9) | CGAL (Nef/Polyhedra) | Truck (Rust B-rep) | Manifold (mesh CSG) |
|---|---|---|---|---|
| Modelling capability: B-rep, fillets, sweeps, STEP (5) | 5 | 2 | 2 | 1 |
| Robustness / numerics (4) | 3 | 4 | 2 | 5 |
| Platforms (3) | 5 | 5 | 4 | 5 |
| Docs (2) | 3 | 4 | 2 | 3 |
| Maintenance (3) | 4 | 5 | 3 | 4 |
| Binary size (1) | 2 | 3 | 4 | 5 |
| Performance (3) | 3 | 3 | 3 | 5 |
| API stability (2) | 3 | 4 | 2 | 4 |
| Testability (2) | 3 | 4 | 3 | 4 |
| Packaging (2) | 3 | 3 | 3 | 5 |
| Bindings (Python) (1) | 4 | 3 | 2 | 4 |
| Replaceability behind adapter (2) | 3 | 3 | 3 | 3 |
| Classroom suitability (licence LGPL-2.1+exception / GPL / Apache / Apache) (2) | 4 | 2 | 5 | 5 |
| Contributor ecosystem (1) | 4 | 4 | 2 | 3 |
| **Total** | **118** | 107 | 85 | 116* |

*Manifold scores high but cannot satisfy MVP B-rep needs (fillets, STEP B-rep export); keep as optional mesh-boolean
helper for STL workflows. **Decision: OCCT primary**, behind `IGeometryKernel`. Alternative if OCCT fails PoC K-2:
evaluate commercial kernels (Parasolid/ACIS licensing) — reversible only before milestone 10.

### J.2 Sketch solver

| Criterion (w) | PlaneGCS (FreeCAD, LGPL) | libslvs (SolveSpace, GPLv3) | Own LM/Newton on Eigen |
|---|---|---|---|
| Constraint coverage incl. tangency, symmetry, equal (5) | 5 | 4 | 2 |
| Redundancy/conflict diagnosis (4) | 4 (DOF + redundant sets) | 4 | 2 |
| Determinism / minimal-change behaviour (F67) (4) | 3 | 3 | 4 |
| Licence fit (3) | 4 (LGPL) | 2 (GPL) | 5 |
| Separability from host app (3) | 3 (extract from FreeCAD) | 4 | 5 |
| Maintenance (2) | 4 | 3 | 2 |
| **Total** | **79** | 69 | 64 |

**Decision:** PlaneGCS in PoC K-1; libslvs as fallback (licence review needed); own solver only for specific
extensions.

### J.3 Remaining stack

| Area | Choice | Alternative | Reversible until |
|---|---|---|---|
| Language | **Rust core (DECIDED 2026-10-12)** — own sketch solver (F11), domain, persistence; OCCT via thin C++ shim (`cxx`) | C++20 core | decided |
| Scripting | Python via PyO3 (decision N.2a-7: API exposed to Python) | — | M2 |
| UI | Qt 6 Widgets (+ QML only for viewport overlays if useful) | TypeScript/WASM web UI (OCCT.js) | M3 |
| Linear algebra | Eigen 3.4 | — | always (internal) |
| Build | CMake + vcpkg (manifest) | Conan 2 | M1 |
| Storage | ZIP package (H.1) via minizip-ng; SQLite only for local index/recent files | SQLite single-file | M4 |
| Scripting | Python 3.12+ embedded, pybind11 | Lua | FPR |
| Tests | Catch2 v3, rapidcheck, libFuzzer | GoogleTest | always |
| Rendering | **wgpu (DECIDED 2026-10-12)** behind `IRenderer`: Vulkan/Metal/DX12 (+GL/WebGPU fallback); GPU ID-buffer picking, shader section planes, instancing, MSAA; tessellation on CPU (OCCT BRepMesh, parallel, background) | OCCT AIS/V3d | M3 |
| Web target | keep core crates (solver, domain, persistence) **wasm32-compatible from day one** (no direct FS/threads/OS calls; IO behind ports; CI builds wasm32); wgpu → WebGPU/WebGL2; kernel via a **remote-kernel adapter** of `IGeometryKernel` (OCCT server) first, Emscripten OCCT (opencascade.js-style) optional later; JS API on web, Pyodide optional | desktop only | design rule now; web edition later |
| GPU compute | optional wgpu compute for large-mesh ops / analyses — never authoritative geometry; solver and kernel stay CPU double precision (determinism) | — | later |
| AI assistance | local inference via ONNX Runtime (`ort`: CUDA / DirectML / CoreML providers) or `candle`; cloud LLM via API; actions go through the Python/command API with undo | — | later phase |
| Units/expressions | own evaluator (grammar decision N.2-6) + mp-units concepts | — | M2 |
| Interchange | OCCT STEP/IGES translators; lib3mf (BSD) for 3MF; own STL/OBJ; DXF via libdxfrw (GPL — review) or own subset | — | per format |

## K. Technical dependency-risk register

| ID | Dependency / area | Function | Req? | Replacement boundary | Known limits | Integration complexity | PoC before adoption | Project health | Risk |
|---|---|---|---|---|---|---|---|---|---|
| K-R1 | OCCT | B-rep kernel, STEP/IGES | required | `IGeometryKernel` | fillet robustness, boolean tolerance issues, large API | high | K-2 (history → labels), fillet suite | active, company-backed (Open Cascade SAS) | **High** |
| K-R2 | Topological naming scheme | stable references (G.4) | required | `ITopologyNamer` | kernel history completeness per op | high | K-2 incl. F42/F43/F44/F116 cases | n/a (own) | **High** |
| K-R3 | PlaneGCS | sketch solver | required | `ISketchSolver` | coupled to FreeCAD types; minimal-change behaviour unknown | medium | K-1 (SK-01…SK-13, 500 entities ≤ 50 ms) | active (FreeCAD) | **High** |
| K-R4 | Profile detection | regions from sketch arrangement | required | own module | tolerance handling, nested loops | medium | K-3 | own | Medium |
| K-R5 | Determinism | identical rebuilds across OSes | required | — | FP differences, threading | medium | K-4 (100×, 2 OSes) | own | Medium |
| K-R6 | Qt 6 | UI | required | presentation layer only | LGPL compliance (dynamic linking), accessibility gaps in custom widgets | medium | accessibility spike (screen reader on ribbon + tree) | active | Medium |
| K-R7 | OCCT visualization | MVP renderer | optional | `IRenderer` | look & feel, picking API | low | viewport spike | active | Low |
| K-R8 | lib3mf | 3MF I/O | required (MVP export) | `ITranslator` | — | low | — | active (3MF Consortium) | Low |
| K-R9 | Python/pybind11 | scripting | optional (FPR) | `IPlugin` | sandboxing | medium | capability model test | active | Medium |
| K-R10 | DXF library | 2D exchange | optional | `ITranslator` | GPL licence of libdxfrw | low | licence review | moderate | Medium |
| K-R11 | vcpkg ports for OCCT/Qt | build | required | build only | port versions lag | low | CI on 3 OSes | active | Low |

Reversible decisions (keep open): renderer, scripting language, DXF library, storage variant (until M4), UI
technology (until M3). Irreversible after M10: kernel and naming scheme — hence K-1/K-2 first (L).
