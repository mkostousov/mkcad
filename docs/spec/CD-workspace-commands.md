# C. Workspace and component hierarchy · D. Command inventory (draft 1)

Status: draft, 2026-10-11. Reference: Fusion Personal 2705.1.30, Windows 10 VM, 1920×1080 @100 %, en-US, default
(uncustomised) profile. Tags: [O] [D] [I] [P] [U]. Full machine-readable lists live in
`vmharness/inventory/` (`workspaces.md`, `commands.csv` — 3122 command definitions, `shortcuts.md`, `menus/`).
Shortcuts are platform- and build-specific; macOS was not tested → every shortcut below is Windows-only.

## C.1 Reference workspace anatomy [O]

| Region | Default location (1920×1080) | Observed behaviour | Ref |
|---|---|---|---|
| Document tabs + QAT | top | title "Untitled*" when dirty; unsaved banner "Unsaved: Changes may be lost — Save" | F7, F32 |
| Workspace selector + ribbon | top, below tabs | 38 workspaces; Design has 16 toolbar tabs incl. contextual ones (SKETCH, BASE FEATURE…, POSITION) | F1 |
| Contextual tab | ribbon | SKETCH tab appears on sketch entry with CREATE, MODIFY, CONSTRAINTS, CONFIGURE, INSPECT, INSERT, SELECT; removed on Finish | F31, F98 |
| Browser (model tree) | left, ≈[4,144 300×817] | root "(Unsaved)" → Document Settings, Named Views, Origin (hidden by default), Sketches, Bodies…; eye toggles; activate radio | F7, F104 |
| Canvas | centre | wheel zoom, middle-drag pan, Shift+middle orbit; right-click → marking menu | F6, F94 |
| ViewCube | top right | right-click menu: Go Home, projection radio (Ortho / Persp / Persp+Ortho faces), Set/Reset Home, Set current view as ▸, Reset Front, ViewCube Size ▸ | F102 |
| Navigation bar | bottom centre ≈[658,960] | Orbit ▾, Look At, Pan, Zoom, Fit ▾, Display Settings ▾, Grids and Snaps ▾, Viewports ▾ | F103 |
| Timeline | bottom | feature icons + marker; item context menu (13 entries) | F101 |
| Command dialog | right side ≈x 1500–1920 | e.g. EXTRUDE task panel, OK/Cancel; inline canvas value field + manipulators | F69 |
| Status bar | bottom right | live selection summary "1 Profile \| Area : 800.00 mm^2" | F68 |
| Sketch Palette | right side in sketch mode | options (Linetype, Look At, grid/snap/profile/points/dims/constraints/projected/construction/slice/3D sketch), Finish Sketch | F33 |
| Command search | popup on `S` | "Design Shortcuts" filtered list; first Esc closes only the result list | F5 |
| Hint | canvas top-left | "Ctrl to modify selection" while a command has a selection input | OCR F112 run |
| Data panel, autosave/recovery UI, progress | [U] | not exercised | — |

Accessibility [O] F7: UIA exposes only region rectangles; ribbon, tree and canvas content are not exposed to
assistive technology in this build. **[P]** Every region of the new product exposes its content through the
platform accessibility API (named controls, tree items, canvas selection as a list).

## C.2 Proposed topology [P]

Keep the reference's learnable layout (ribbon tabs per modelling mode, browser left, timeline bottom, view cube
top-right, command panel right) because it transfers to other professional CAD systems; change:
1. One command panel location, docked, keyboard reachable (Tab cycles fields; Enter = OK; Esc = Cancel, F72).
2. Status bar always shows: active command prompt, selection summary (F68), sketch DOF (artifact F §F.6).
3. Diagnostics panel listing all Warning/Error features with codes and "explain" links (learning mode).
4. Ribbon shows only commands the licence/document allows; hidden-but-defined commands are a reference quirk (F30).
5. Layout persistence per user profile; reset-to-default command.

## D.1 Command tree format

```text
Workspace
└── Tab
    └── Panel
        └── Command — shortcut (Windows) · availability · selection · surface · class/evidence
```

## D.2 Design workspace — verified subset

```text
Design
├── SOLID
│   ├── CREATE
│   │   ├── Create Sketch — no shortcut shown · needs planar support (origin planes offered) · task in canvas · [O] F31
│   │   ├── Extrude — E (shown in menu; pressing E opened Extrude with a preselected profile [O] F113 run)
│   │   │       · profile/face selection · task panel + canvas manipulators · [O] F69
│   │   └── Hole — H · [O] menu label only, key not pressed
│   ├── MODIFY — Press Pull Q · Fillet F · Move/Copy M · Delete Del · Appearance A · Compute All Ctrl+B · [O] labels
│   ├── ASSEMBLE — Joint J · As-Built Joint Shift+J · [O] labels
│   ├── INSPECT — Measure I · Display Component Colors Shift+N · Display Mesh Face Groups Shift+F · [O] labels
│   ├── SELECT — Window 1 · Freeform 2 · Paint 3 · [O] labels
│   └── CONSTRUCT / CONFIGURE / INSERT — no shortcuts shown · [O]
├── SKETCH (contextual)
│   ├── CREATE — Line L, 2-Point Rectangle R, Center Diameter Circle C, Project P — [O] keys verified via
│   │           activeCommand F138 (R, C, P not labelled in menus) · full tool list F34
│   ├── MODIFY — Fillet, Chamfer ×3, Blend Curve, Offset (O), Trim (T), Extend, Break, Sketch Scale, Move/Copy (M), Change Parameters · [O] API tree F34, keys F138
│   ├── (solid keys A, E, Q, F work inside a sketch and run the solid command; sketch resumes after Esc [O] F139)
│   ├── CONSTRAINTS — AutoConstrain + 14 constraints, no shortcuts shown · [O] F35
│   └── Sketch Dimension — D · [O] used F37
├── MANAGE — BOM ▾ → Bill of Materials (Personal licence) · [O] F110
└── UTILITIES — Scripts and Add-Ins Shift+S · [O] label
Global — S: command search [O] F5 · Ctrl+Z / Ctrl+Y undo/redo [O] F48, F111 · Enter/Esc OK/Cancel [O] F72, F96, F97 ·
         Delete removes selection [O] F95 · Ctrl+Shift+F full screen [O] label F103, F156 · V show/hide [O] label F154 ·
         Ctrl+C / Ctrl+X on bodies [O] label F151 · Ctrl held while drawing suppresses inference [O] F164
```

Context-menu placements [O]: canvas marking menu F94 (sketch mode); timeline item F101, F149; ViewCube F102, F155;
browser — body F151, sketch F152, folders F153; navigation bar Grid and Snaps / Display Settings F156.

Selection [O] F147/F148: click priority vertex > edge > face with ≈ 8 px vertex band and ≈ 6 px edge band.

Gaps [U]: SURFACE/SHEET METAL/PLASTIC/UTILITIES SELECT panels, MESH INSERT, PLASTIC CONSTRUCT, UTILITIES MAKE;
Viewports ▾; root/Document Settings browser menus; data panel; macOS shortcuts entirely.

## D.2a Drawing workspace (later phase) [O] F216, F238–F245

```text
Drawing (document type; created from a saved design: Create Drawing / New… ▸ Drawing)
├── Create Drawing dialog — Automatic | Manual, Standard ISO, Units mm, Sheet A3 420×297, sheet count, automation sheets
├── DRAWING tab
│   ├── CREATE — Base, Projected, Auxiliary, Section, Detail, Break, Breakout Section, Crop, Perspective View; Create Sketch
│   ├── MODIFY — Rotate, Move, Delete (Del), Refresh Properties
│   ├── GEOMETRY — Center Line, Center Mark, Center Mark Pattern, Edge Extension
│   ├── DIMENSIONS — Dimension, Auto Dimension (alternative schemes), Tidy Up, Ordinate/Linear/Aligned/Angular/Radius/
│   │                Diameter/Jogged Radial/Arc Length/Curve Min Max/Baseline/Chain, Arrange, Match, Flip Arrows, Break
│   ├── TEXT — Text, Note, Leader Note, Hole and Thread Note, Bend Note
│   ├── SYMBOLS — Surface Texture, Feature Control Frame, Datum Identifier, Welding, Taper and Slope, Edge Symbol, Sketch Symbol
│   ├── INSERT / TABLES / INSPECT — not captured
│   └── EXPORT — PDF (face button; Sheets, Open PDF, Lineweights) · DWG · Sheet as DXF · CSV
├── Base view dialog — Orientation (Front), Style ×4, Scale 2:1, edge visibility, automated centre marks/lines
└── Title block auto-filled (author, date, title = design name); PDF output A3 landscape PDF 1.7
```
[P] MVP excludes drawings (A.3); first production release targets base/projected/section views, overall + chain
dimensions, title block, PDF + DXF export. Projection convention must follow the sheet standard explicitly (ISO =
first angle) — the reference placed a top view above the front view on an ISO sheet (F242, to be confirmed).

## D.3 Proposed MVP command set [P]

| Area | Commands (MVP) | Shortcut proposal |
|---|---|---|
| Sketch | Create/Finish Sketch, Line, Rectangle (2-pt, centre), Circle (centre-dia), Arc (3-pt, centre), Point, Construction toggle, Trim, Extend, Offset, Fillet, Mirror, Project | adopt verified reference letters L, R, C, D, T, O, P, M (F138); X for construction toggle [P] (reference: X with no selection does nothing, F140); Arc: no reference key — propose none (A is Appearance in the reference, F139) |
| Constraints | Coincident, H/V, Parallel, Perpendicular, Tangent, Equal, Concentric, Midpoint, Collinear, Symmetric, Fix, Dimension | D for dimension (as reference) |
| Features | Extrude, Revolve, Fillet, Chamfer, Shell, Hole, Rectangular/Circular pattern, Mirror | E, F, H as reference; others none by default |
| Construct | Offset plane, Plane at angle, Axis through two points | — |
| Inspect | Measure, Section analysis | I as reference |
| Global | Undo, Redo, Search, Delete, Select filters | Ctrl+Z, Ctrl+Y, S, Del, 1/2/3 selection modes as reference |

Shortcuts differing from the reference must be documented as such in the learning material.
