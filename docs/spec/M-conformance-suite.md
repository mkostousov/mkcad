# M. Black-box conformance test suite (draft 1)

Status: draft, 2026-10-11. Tests are written against the NEW product's public behaviour; the "Ref" column cites the
reference observation that motivated the expectation (Fnn) or marks the expectation as a deliberate improvement [P].
Every test asserts diagnostic **codes** (H.5), never display strings. Default tolerances: length 1e-6 mm, area/volume
relative 1e-12 (unless stated). Harness: headless driver issuing CommandBus requests (H.4.3) + UI-level tests for rows
marked (UI).

Columns: ID · input actions · expected logical state · observable feedback · tolerance · pass/fail · Ref.

## M.1 Sketch and solver (SK) — defined in artifact F.9, normative here

| ID | Input actions | Expected logical state | Feedback | Tol | Pass if | Ref |
|---|---|---|---|---|---|---|
| SK-01 | 2-pt rectangle from origin; W=40, H=20 | 4 lines, 4 shared points, 2H+2V, fully constrained, 1 profile 800 mm² | DOF 0 shown | 1e-6 | all equal | F14, F36, F68 |
| SK-02 | SK-01; W 40↔73.3 ×20 | 1 distinct geometry at W=40 | — | 0 (bit) | set size 1 | F16 |
| SK-03 | SK-01; dimension opposite edge | (UI) dialog offering driven; accept → driven dim; API → `SKETCH.CONSTRAINT.REDUNDANT`, no change | driven shown in parentheses | — | codes match | F15, F38 |
| SK-04 | SK-01; W=0, W=−10 (UI and API) | refused, doc unchanged, undo depth unchanged | field error state | — | `SKETCH.DIM.INVALID_VALUE` both paths | F39, F17/F18 [P] |
| SK-05 | triangle 30/20/20 → 30/10/10 | `SKETCH.SOLVE.CONFLICT`; previous geometry+expressions kept | — | 0 | unchanged | F105 |
| SK-06 | triangle 30/15/15 | solved collinear + Warning degenerate | warning | 1e-6 | code present | F106 [P] |
| SK-07 | alternate side 20↔28 ×10 | apex stays y>0; exactly 2 positions | — | 1e-9 | | F107 |
| SK-08 | each constraint on unconstrained geometry | movement per F.3 table (first entity stays; rotate about midpoint; length preserved) | — | 1e-6 | | F61–F65 |
| SK-09 | same constraint twice | `SKETCH.CONSTRAINT.DUPLICATE`, no change | — | — | | F60 |
| SK-10 | chain gap 1e-5 / 2e-5 mm | 1 / 0 profiles | — | — | | F76 |
| SK-11 | squares 1e-3, 1e6, offset 1e6 mm | exact areas | — | rel 1e-12 | | F77 |
| SK-12 | rect ⊃ circle ⊃ circle | 3 profiles, middle 2 loops | — | — | | F73 |
| SK-13 | Ctrl+Z ×5 / Ctrl+Y ×5 after rect+dim+value+driven+sketch | reverse order exactly; redo bit-identical | — | 0 | | F48, F49 |
| SK-14 | (UI) line end 5 px / 12 px off horizontal | H constraint / none (thresholds per F.4 decision) | inference glyph | px | | F87 |
| SK-15 | project edge; delete source | curve kept; `SKETCH.REF.LOST` | warning badge | — | | F84 |
| SK-16 | under-constrained rect; W 60↔40 ×10 | result reproducible on re-run from same file (determinism), may differ from start | — | 0 | identical across runs | F10 [P] |
| SK-17 | fix a line; add Horizontal on oblique fixed line | `SKETCH.CONSTRAINT.CONFLICT` | — | — | | F66 |
| SK-18 | symmetric lines about axis | second line on mirror of first's infinite line (or endpoint mirror per N.2-2) | — | 1e-6 | per decision | F65 |

## M.2 Transactions and timeline (TX) — from artifact E.3

| ID | Input actions | Expected | Feedback | Tol | Ref |
|---|---|---|---|---|---|
| TX-01 | open Extrude, enter 10, Cancel | timeline, bodies, undo depth unchanged; selection cleared | — | 0 | F72 |
| TX-02 | Extrude 10 + Enter | +1 timeline item; selection empty | — | — | F72 |
| TX-03 | insert hole before fillet via rollback | fillet Healthy, same edge label | — | — | F41 |
| TX-04 | notch removes filleted edge; delete notch | `REF.LOST` (Error, cached geometry) → Healthy, identical volume | health badge | rel 1e-12 | F43, F44 |
| TX-05 | reorder dependent before parent | `GRAPH.REORDER.DEPENDENCY`, no change | — | — | F24 |
| TX-06 | suppress / unsuppress fillet | identical volume | — | rel 1e-12 | F23 |
| TX-07 | delete base feature | dependents listed for confirmation; no orphan without diagnostic | dialog | — | F27 [P] |
| TX-08 | Finish Sketch then Undo | sketch edit re-entered | — | — | F99 |
| TX-09 | A=B then B=A | `PARAM.EXPR.CYCLE`; A unchanged | — | — | F55 |
| TX-10 | export STEP | dirty flag unchanged; header schema as configured | — | — | F46 |
| TX-11 | extrude 10→1 mm under r=2 fillet | `FEATURE.RESULT.TOPOLOGY_CHANGED` when body count changes | warning | — | F25 [P] |
| TX-12 | cut circle off the face | `FEATURE.RESULT.NO_EFFECT` | warning | — | F26 [P] |
| TX-13 | marker move | is an undo step; undo restores marker | — | — | F111 |
| TX-14 | in-dialog undo inside open command | affects command inputs only; document stack untouched | — | — | F112 |
| TX-DET | rebuild same package 100× on Windows + Linux | bit-identical solved coords; volumes rel 1e-12 | — | see | K-4 |

## M.3 Expressions & units (EX, UN)

| ID | Input | Expected | Ref |
|---|---|---|---|
| EX-01 | `Len + 1 in` (Len=40 mm) | 65.4 mm | F52 |
| EX-02 | `Len / 1 mm` | 40 (unitless) | F52 |
| EX-03 | `max(Len, 10 mm)` (separator per N.2-6) | 40 mm | F53 [P] |
| EX-04 | `PI`, `pi` | 3.14159…; `pi` → `PARAM.EXPR.INVALID` (no silent 0) | F56 [P] |
| EX-05 | name `W`, `A`, `1abc`, `with space`, duplicate | `PARAM.NAME.INVALID` with reason reserved_unit / syntax / duplicate | F54 [P] |
| EX-06 | `40 kg` for length | `PARAM.EXPR.INVALID` reason unit_kind | F55 |
| EX-07 | self reference `A = A + 1 mm` | `PARAM.EXPR.CYCLE` | F55 |
| EX-08 | delete parameter in use | `PARAM.IN_USE`, not deleted | F58 [P] |
| EX-09 | dependent update Len 40→100 | dimension driven by `Len/2` = 50 mm, geometry follows | F57 |
| EX-10 | `2 in` stored | expression text preserved after save/open | F20 |
| EX-11 | unitless `10` in length field (display mm) | 10 mm | F71 |
| EX-12 | 10k random valid expressions | evaluator = reference big-decimal oracle within 1e-12 rel | [P] |
| UN-01 | switch display unit mm→in→mm | stored expressions unchanged; geometry unchanged | F182 |
| UN-02 | model dimension display after switch | shown in document unit; user parameters keep their declared unit | F182 |
| UN-03 | unitless entry "10" in a length field | stored with the current display unit appended ("10 mm") | F183 |
| UN-04 | unit switch is one undo step; no other undo entries | [P] | G.6 |

## M.4 Viewport & navigation (VW), inference (IN)

| ID | Input | Expected | Ref |
|---|---|---|---|
| VW-01 | wheel ×5 | zoom about cursor | F6 |
| VW-02 | MMB drag | pan by drag vector (screen-space exact) | F6 (300,−100 px) |
| VW-03 | Shift+MMB drag | orbit | F6 |
| VW-04 | view cube face click | orthographic named view | F104 |
| VW-05 | enter sketch | view normal to sketch plane (configurable) | F31 |
| VW-06 | pick on 1M-triangle model | correct entity < 16 ms | [P] |
| IN-01…IN-08 | endpoint snap 8 px, H/V ~9 px, on-curve 2 px, midpoint, perpendicular, tangent glyph, Ctrl suppresses inference, zoom-independence | per F.4 / N.2-1 decision | F85–F93 |

## M.5 Persistence (PK)

| ID | Input | Expected |
|---|---|---|
| PK-01 | save → open | authoritative parts byte-identical after canonical JSON |
| PK-02 | delete `cache/` → open | rebuild equals original within TX-DET tolerances |
| PK-03 | kill process during save | previous file intact |
| PK-04 | truncate one sketch JSON | that sketch Error, others load (H.6) |
| PK-05 | open schema 1.x fixture in 2.0 | migration logged, backup created |
| PK-06 | open newer major | `DOC.SCHEMA.NEWER`, read-only |
| PK-07 | unknown fields | preserved on save |
| PK-08 | autosave interval + crash | recovery offers autosave; no data newer than interval lost |

## M.6 Features (EXT, FT) and references (REF)

| ID | Input | Expected | Ref |
|---|---|---|---|
| EXT-01 | extrude dialog defaults | One Side, Distance, taper 0, op New Body at d=0 | F69, F143 |
| EXT-01a | enter d so preview overlaps body / only touches a face / touches an edge or nothing | op auto = Cut (+ "objects to cut" input) / Join / New Body; user override sticks for the rest of the command [P] | F144, F145 |
| EXT-02 | distance 0 | OK disabled / `FEATURE.PARAM.INVALID` | F69 |
| EXT-03 | symmetric 10 mm, "full length" on/off | 4000 / 8000 mm³ (20×20 profile) | F175 |
| EXT-04 | two sides 10 + 5 | 6000 mm³ | F175 |
| EXT-05 | taper ±10°, ±45° | positive = outward frustum (sign convention documented; reference F176) | F176 |
| EXT-06 | inward taper whose sides meet before the distance | `FEATURE.PARAM.INVALID` (or Warning `FEATURE.RESULT.CLAMPED` if clamping is chosen) — never silent | F177 [P] |
| EXT-07 | through-all cut | exact hole volume | F175 |
| EXT-08 | to object (plane at 25 mm) | 10000 mm³; follows the plane when it moves | F175 |
| EXT-09 | intersect | 1000 mm³ | F175 |
| EXT-10 | thin extrude | wall volume analytic | [P] |
| FT-01 | fillet 4 top edges of 40×20×10 box, r 2 mm | volume = 8000 − 4·(1−π/4)·r²·L_i analytic; Healthy | F168 |
| FT-02 | same, r = height / r > height | `FEATURE.RESULT.TOPOLOGY_CHANGED` warning or `FEATURE.PARAM.INVALID` — never silent | F168 [P] |
| FT-03 | fillet all 12 edges r 6 (> half height) | `KERNEL.OP.FAILED` (corner) ; document unchanged | F169 |
| FT-04 | chamfer > height | `KERNEL.OP.FAILED` (size); document unchanged | F169 |
| FT-05 | shell open top t = half width | `KERNEL.OP.FAILED` (offset); t slightly smaller succeeds | F169, F170 |
| FT-06 | revolve 360° profile clear of / touching / crossing the axis | ring π(r₂²−r₁²)h / solid πr²h / `KERNEL.OP.FAILED` reason profile_crosses_axis, atomic | F171 |
| FT-07 | body pattern with overlapping instances | either merge per user option or Warning `FEATURE.RESULT.OVERLAP` [P] — never silent overlapping solids | F173 [P] |
| FT-08 | mirror body with plane touching / cutting it | same rule as FT-07 | F174 [P] |
| FT-09 | pattern dialog/API defaults | one direction unless a second is specified explicitly | F172 [P] |
| FT-10 | simple hole d10 depth 5 | cylinder + 118° drill tip (tip angle a documented hole parameter) | F178 |
| FT-11 | counterbore through | two exact cylinders, no tip | F178 |
| FT-12 | draft 5° four sides | analytic frustum | F179 |
| FT-13 | draft where sides would cross | `FEATURE.PARAM.INVALID` or Warning `FEATURE.RESULT.CLAMPED` — never silent | F179 [P] |
| FT-14 | sweep profile along line | πr²L exactly | F180 |
| FT-15 | sweep with profile far from path | Warning `FEATURE.INPUT.DISJOINT` [P] (reference silent) | F180 [P] |
| FT-16 | loft two equal sections | prism volume | F181 |
| FT-17 | loft with one section | `FEATURE.PARAM.INVALID`, OK disabled | F181 |
| FT-18…20 | shell variants, combine, mirror features (not bodies) | per F166 dialogs | F166 |
| REF-01 | unrelated upstream insert | exact | F41 |
| REF-02 | edge split | expanded(2) | F42 |
| REF-03 | edge removed | lost → recovers | F43, F44 |
| REF-04 | sketch-on-face, face resized | sketch follows | F28 |
| REF-05 | face deleted under sketch | `SKETCH.REF.LOST`, cached frame | F27 |
| REF-06 | split region used by profile | expanded or `REF.AMBIGUOUS` (never silent pick) | F116, F119 [P] |
| REF-07 | shell face split upstream | both pieces | F118 |
| REF-08 | split edge re-merged | set shrinks back | F124 |

| CN-01 | offset plane ±5 mm from a face | along outward normal; negative inward | F196 |
| CN-02 | plane at angle about an edge with reference face | rotation sense documented (reference tilts outward for +) | F196 |
| CN-03 | midplane of parallel / non-parallel faces | mid-distance / bisector; [P] show which in the dialog | F197 |
| MS-01 | measure min distance between two boxes 30 mm apart (face/body/vertex) | 30 mm (abs 1e-6) | F195 |
| MS-02 | angle between opposite-facing parallel faces | 0° as plane angle; [P] also show normal angle 180° in learning mode | F195 |
| MS-03 | mass of 40×20×10 steel box | 62.8 g at 7.85 g/cm³ | F185 |

| SM-01 (later phase) | base flange from 40×20 profile, rule t 2.5 / k 0.44 / r = t | 40×20×2.5 plate | F203 |
| SM-02 | edge flange 10 mm (outer datum, bend inside) | 40×20×10 part | F204 |
| SM-03 | flat pattern of SM-02 | 40 × 25.655 mm (K-factor allowance, abs 1e-3) | F206 |
| JT-01 (later phase) | revolute joint face-centre to face-centre; drive 90° | child moves, joint Healthy; limit exceeded → clamp to limit + warning [P] (reference reads 0, F200) | F199, F200 |

| DR-01 (FPR) | base view of 50×20×10 part, front, 2:1, A3 ISO | view outline 100×20 mm on sheet; title block filled from document metadata | F240 |
| DR-02 | projected views right / above on an ISO sheet | first-angle placement per ISO 128-30 (reference placed top view above, F242) | [P] |
| DR-03 | overall auto-dimension | 50, 10 (front), 20 (side); values follow model edits on refresh | F243 |
| DR-04 | export PDF | 1 page per sheet, A3 MediaBox 1191×842 pt, vector geometry | F245 |

## M.7 Interchange (IO) and accessibility (AX)

| ID | Input | Expected |
|---|---|---|
| IO-01 | export STEP AP242 box | validates in two independent STEP checkers; volume 8000 mm³ after re-import |
| IO-02 | export STEP AP214 option | header `AUTOMOTIVE_DESIGN` (cf. F46) |
| IO-03 | STL binary/ASCII | binary default; 12 triangles for a box (F47) |
| IO-04 | 3MF | passes 3MF consortium validator; OPC parts as F47 |
| IO-05 | OBJ | with .mtl |
| IO-06 | STEP import multi-body | one body per solid, timeline item "Import", units preserved (X-15) |
| IO-07 | IGES import | legacy surfaces imported or `IO.FORMAT.UNSUPPORTED` per entity type |
| IO-08a | DXF import with $INSUNITS mm / inch / absent | 40 mm / 1016 mm / document unit; sketch named after layer; curves not fixed (reference F220, F221) |
| IO-08 | DXF/SVG sketch import | entity subset list honoured; SVG physical units (`width="40mm"` + viewBox) respected → 40×20 mm (reference uses 96-dpi px, F193 — not adopted); imported curves not silently fixed unless the user chooses [P] |
| IO-09 | cancel 50 MB import | `JOB.CANCELLED`, document unchanged |
| IO-10 | corrupt file | `IO.FILE.CORRUPT`, no crash (fuzz corpus) |
| AX-01…08 | screen reader reads ribbon commands, browser tree items, timeline items, dialog fields, diagnostics; full keyboard path sketch→extrude; high-contrast theme; 200 % scaling | all reachable/announced (reference exposes none of ribbon/tree/canvas to UIA, F7) |
