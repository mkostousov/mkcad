# S. Silent-outcome requirements (draft 3 addendum to A, E, H, M)

Status: 2026-10-12. Every row is a reference behaviour that completes **without any diagnostic** although the result
differs from what the user asked for or is unusual. Product rule [P]: none of these may be silent — each raises the
listed code (H.5), shows it in the diagnostics panel and the learning layer, and is covered by the listed test (M).

| # | Reference outcome [O] | Evidence | Required behaviour [P] | Code | Test |
|---|---|---|---|---|---|
| S-01 | Rebuild splits one body into two (fillet larger than height after an upstream edit) | F25 | Warning naming the feature and the new body count | `FEATURE.RESULT.TOPOLOGY_CHANGED` | TX-11 |
| S-02 | Cut that removes nothing (circle moved off the part) | F26 | Warning; feature kept | `FEATURE.RESULT.NO_EFFECT` | TX-12 |
| S-03 | Fillet radius ≥ part height accepted, part "rolled over" | F168 | Reject at creation, or Warning if produced by an upstream edit | `FEATURE.PARAM.INVALID` / `.TOPOLOGY_CHANGED` | FT-02 |
| S-04 | Taper / draft sides meet before the requested length → solid silently truncated at the apex | F177, F179 | Reject with the max feasible angle/length in the message (or Warning if clamping chosen) | `FEATURE.PARAM.INVALID` / `.CLAMPED` | EXT-06, FT-13 |
| S-05 | Pattern / mirror instances overlap the source, stay separate overlapping bodies | F173, F174 | Warning + offer "combine" | `FEATURE.RESULT.OVERLAP` | FT-07, FT-08 |
| S-06 | Sweep profile far from its path still sweeps | F180 | Warning (profile not on path) | `FEATURE.INPUT.DISJOINT` | FT-15 |
| S-07 | Angle dimension 0°/180°/−10° collapses the line to ≈0.1 mm, not restored | F134 | Reject degenerate angles; rigid rotation keeps length | `SKETCH.DIM.INVALID_VALUE` | SK-04 (ext.) |
| S-08 | Sketch rotate deletes H/V constraints; sketch scale deletes driving dimensions | F137, F207 | List removed items, allow undo; scale updates dimension values instead | `SKETCH.CONSTRAINT.REMOVED` | new SK-19 |
| S-09 | Split support face → profile re-resolves to one piece (part of the cut lost), choice not reproducible | F116, F119 | Expand to all pieces or Warning, never a silent pick | `REF.AMBIGUOUS` | REF-06 |
| S-10 | SVG imported at 96-dpi px ignoring mm units (40 mm → 10.58 mm); curves locked | F193, F223 | Honour physical units; report scale used; curves free unless chosen | `IO.UNITS.ASSUMED` (info) | IO-08 |
| S-11 | API accepts 0 / negative sketch dimensions (GUI refuses) | F17, F18, F39 | Same validation in API and GUI | `SKETCH.DIM.INVALID_VALUE` | SK-04 |
| S-12 | Lower-case `pi` evaluates to 0 | F56 | Unknown identifier error | `PARAM.EXPR.INVALID` | EX-04 |
| S-13 | Deleting a parameter in use returns false silently | F58 | Explicit error listing dependents | `PARAM.IN_USE` | EX-08 |
| S-14 | Unsaved edits lost after a crash although a recovery file had been written | F215 | Offer recovery on next start; never delete recovery data before the user decides | `DOC.RECOVERY.AVAILABLE` (info) | PK-08 |
| S-15 | Joint limit exceeded → value reads 0 instead of the limit | F200 | Clamp to the limit + Warning | `JOINT.LIMIT.CLAMPED` | JT-01 |
| S-16 | Moving a fully constrained sketch point via API "succeeds" but nothing moves | F19 | Return a constrained-move result | `SKETCH.MOVE.CONSTRAINED` | SK-17 (ext.) |

New codes introduced here (append to H.5): `IO.UNITS.ASSUMED`, `DOC.RECOVERY.AVAILABLE`, `JOINT.LIMIT.CLAMPED`;
new test SK-19 (rotate/scale with constraints/dimensions present).
