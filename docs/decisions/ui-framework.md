# Decision: UI Framework (M1)

**Status:** Applied recommendation, needs-user-review

**Date:** 2026-10-14 | **Revision:** 1

---

## Requirement Summary

From `docs/spec/N-open-questions.md` (N.2a), `docs/spec/IJK-architecture-tech-risk.md` (J.3, K-R6), and `docs/PLAN-next.md`:

| Criterion | Requirement |
|---|---|
| **Accessibility** | AX-01…08: screen reader (ribbon, tree, timeline, dialogs), keyboard navigation, 200% scaling, high-contrast |
| **Web/WASM** | wasm32-compatible solver/domain/app core from day one; web edition via WASM later |
| **Rendering** | wgpu integration (M3 viewport: Vulkan/Metal/DX12/GL via WebGPU fallback) |
| **UI density** | Ribbon/toolbar, tree browser, timeline, property dialogs, docking/panels (CAD-style) |
| **Internationalization** | Multi-language support (deferred, but must not block) |
| **Rust stack** | Pure Rust (no C++/CLI boundary at UI layer); LGPL-compatible licensing |

---

## Framework Comparison

| Aspect | egui (eframe) | Slint | Dioxus |
|---|---|---|---|
| **Web/WASM** | ✓ Native wasm32, proven | ✓ wasm32 to canvas | ✓ Wasm via trunk/parcel |
| **Accessibility** | ✓ AccessKit (Windows/macOS); expanding | ✗ None on web; docs recommend against | ⚠ WebView-based (limited for dense UI) |
| **wgpu integration** | ✓ egui-wgpu mature backend | ⚠ Partial (skia-gl, not native) | ⚠ WGPU desktop in progress (v0.5+) |
| **Dense CAD UI** | ⚠ Immediate mode (verbose property grids) | ✓ Declarative DSL (ideal) | ⚠ React-like (moderate friction) |
| **Docking/panels** | ⚠ Requires custom state | ⚠ Requires custom impl | ⚠ Requires custom impl |
| **Maturity** | ✓ Stable; used in Rerun, Bevy tools | ⚠ Production-ready (GL), web not recommended | ⚠ 0.5–0.6 range; desktop WGPU unproven |
| **Licensing** | ✓ Apache 2.0 / MIT | ✓ GPL 3.0 / MIT (permissive option) | ✓ MIT |
| **Ecosystem** | ✓ Large community; many widgets | ⚠ Growing | ⚠ Emerging |
| **i18n friction** | Low (string interpolation) | Medium (DSL translation) | Low (HTML/Rust strings) |

---

## Recommendation: **egui (eframe)**

### Reasons

1. **Accessibility leadership:** AccessKit integration is the most mature cross-platform solution; matches AX-01…08 requirements for screen readers + keyboard + scaling.
2. **Web-first design:** wasm32 support is native and proven; solver/domain cores stay compatible automatically.
3. **wgpu alignment:** egui-wgpu is a well-tested, maintained backend; composing wgpu viewport into eframe is demonstrated in production.
4. **Risk-manageable:** Immediate mode verbosity for property grids is a solvable design problem (scoped components, retained-mode wrappers for panels). Slint's web accessibility ban is non-recoverable.

### Main Risk

**Dense UI friction:** property grids, timelines, and docking require careful component composition. Immediate mode can lead to boilerplate. Mitigated by: (a) building reusable panel components early (M1), (b) not adopting declarative-only mindset, (c) custom renderer for specific dense regions if needed.

### Path to Web Edition

Solver + domain already wasm32 by decree. UI: eframe on web via `eframe_template` or custom wasm runner; viewport renderer swaps to WebGPU. No framework change needed later.

---

## 1-Day Spike Verification

To confirm this recommendation before M1 commitment, the spike must:

1. **Property grid:** Build a mock `FeatureProperties` panel (name, value, unit, driving/reference toggles) in egui; measure LOC and perceived UX.
2. **AccessKit:** Enable on Windows; verify screen reader announces controls in eframe's demo app.
3. **wgpu embedding:** Render a simple egui window with a wgpu canvas interior (borrowed from `egui-wgpu` examples).
4. **Web build:** `cargo build --target wasm32-unknown-unknown`; measure size and check browser startup latency.
5. **Panel docking proof:** Sketch one dockable sidebar's state management (not full implementation).

**Exit criterion:** All 5 tasks complete with no blockers; document trade-offs observed (e.g., "property grids need 30 lines of immediate-mode code; acceptable").

---

## Deferred

- Slint reconsideration: only if egui's accessibility maturity stalls and web edition is descoped.
- Custom renderer (e.g., iced, Bevy UI): only if egui's dense-UI friction causes milestone slip.

