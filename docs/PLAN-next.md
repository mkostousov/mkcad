# Next-phase plan (2026-10-12)

State: reference study complete for MVP + first release (evidence F1–F247 in `STUDY.md`, spec A–N draft 2 in
`docs/spec/`). Trial licence active (AutoConstrain, IGES/SAT, drawings in VM with 3D off). Remaining reference gaps are
low value (sketch Mirror GUI, Viewports menu, some SELECT/MESH menus, macOS, section/detail drawing views).

## Phase 1 — Spec draft 3 (analysis, no VM) · DONE 2026-10-12 except item 4 (needs the user) — see spec/CHANGELOG.md
1. Fold the trial findings into A.3 scope (drawings FPR detail, AutoConstrain → learning-mode "suggest constraints"),
   E (drawing creation + export state machines), H (drawing/sheet/view/title-block contracts, PDF/DXF/DWG jobs).
2. Consolidate the "silent outcome" catalogue (README §1.12) into one requirement table with codes and tests.
3. Resolve or explicitly defer every remaining [U] row; produce `spec/CHANGELOG.md`.
4. Decide N.2 product decisions 1–7 with the user (one short review session).

## Phase 2 — Stack decision (needs the user) · blocking for Phase 3
The spec's baseline (J) is C++20 + Qt 6 + OCCT + PlaneGCS (brief's hypothesis). The user's own projects use
.NET 10 / C# (MK.* style). Options to decide before any PoC code:
| Option | Core | Kernel | Solver | UI | Notes |
|---|---|---|---|---|---|
| A (brief baseline) | C++20 | OCCT native | PlaneGCS | Qt 6 | strongest ecosystem fit; new language stack for the user |
| B (.NET-centric) | C# .NET 10 | OCCT via C++/CLI or a thin C ABI wrapper | PlaneGCS via C ABI or own C# LM solver | WPF/Avalonia | matches user's style; P/Invoke boundary must stay coarse (K-R1/K-R3 risk up) |
| C (hybrid) | C++ geometry/solver core as a library | OCCT | PlaneGCS | .NET UI over a C ABI | clean ports-and-adapters split (I.3) at the language boundary |

| D (Rust) | Rust | OCCT via `cxx` bridge (or Truck/Fornjot — too immature for fillets/STEP) | own solver in Rust | egui/Slint, or .NET/Qt over C ABI | memory-safe, great tooling/determinism; OCCT bridging is the main cost; small CAD ecosystem |

### DECIDED 2026-10-12 (user): core language **Rust**; own 2D sketch solver in Rust (F11); OCCT (LGPL 2.1 +
Open CASCADE exception, dynamic linking) via a thin C++ shim; UI to be decided later (Python scripting API required).

### Build vs reuse (recommendation 2026-10-12)
- **2D sketch constraint solver: build our own** (Newton/Levenberg–Marquardt on a sparse Jacobian + rank/DOF
  analysis). Reasons: it is the heart of the teaching product (DOF explanations, minimal-change policy F67,
  deterministic results F16, rich diagnostics S-07/S-16) and is ~3–6 k lines; PlaneGCS stays as a comparison oracle.
- **3D B-rep kernel: do NOT build** — fillets, booleans, shells, STEP are decades of work; use OCCT behind
  `IGeometryKernel` (J.1). Revisit only if OCCT fails PoC K-2.
- **Language**: the solver PoC can be written in the chosen core language immediately; the kernel decision only
  affects how OCCT is bound. If Rust is chosen, the solver and domain core in Rust + OCCT via a thin C++ shim is the
  cleanest variant of option C.

## Phase 3 — Proofs of concept (order from spec README §2)
1. K-2 topological naming on OCCT history (REF-01…08) · 2. K-1 solver SK-01…18 · 3. K-4 determinism ·
4. K-3 profile detection · then fillet robustness, accessibility spike, STEP AP242 round-trip.
Each PoC: own folder under `mk_cad/poc/`, test-first against artifact M rows, results logged in `poc/RESULTS.md`.

Status (2026-10-12):
- **K-1 solver — PASSED.** `poc/sketch-solver/` (Rust): all SK rows in scope (SK-01…09, 13, 16…19, S-07, S-16,
  constraint matrix F60–F67 with reference coordinates), 25 DOF samples, ≤ 25 ms worst case for 500 curves /
  ~600 constraints (limit 50 ms), bit-identical results Windows MSVC vs Linux (WSL2), wasm32 build OK.
  Details, decisions (accepted 2026-10-14) and follow-ups: `poc/RESULTS.md` § K-1. SK-10/11/12 → K-3, SK-14 → UI, SK-15 → K-2.
- Next: K-2 topological naming on OCCT (needs the `occt-sys`/kernel crate), K-3 profile detection (can reuse the
  sketch model of K-1), K-4 determinism at document level (the K-1 golden-digest pattern extends to it).

- 2026-10-13: OCCT build started detached: vcpkg at `C:\dev\vcpkg`, `vcpkg install opencascade:x64-windows`
  (dynamic), log `C:\dev\vcpkg-occt.log` (last line "exit 0 end …" = done). Next session: K-2 kernel spike
  (`occt-sys` + `kernel` crates via cxx; CARGO_TARGET_DIR outside SynologyDrive, e.g. `C:\dev\cargo-target`).

## Housekeeping
- Saved document `mk_x16_persist` (Default Project) and unsaved drawing in the VM — delete only on user request.
- VM: keep 3D OFF (drawings), 4 vCPU, DxfTranslator runtime copy in place (`C:\mkprobe\dxf_runtime_copied.txt`).
- Usage pacing: this chat's context is ~800k tokens; Phase 1 writing is cheaper in a compacted/fresh context.
