# MKProbe job: solver acceptance cases (§4.8) — triangle by 3 lengths (feasible / impossible / degenerate),
# branch stability under repeated edits, tangency chain under edits, near-degenerate constraints.
P3 = adsk.core.Point3D.create
doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
out = {}


def mm(v):
    return v / 10.0


def pt(p):
    return [round(p.geometry.x * 10, 4), round(p.geometry.y * 10, 4)]


def setv(d, expr):
    try:
        d.parameter.expression = expr
        return "ok"
    except Exception as ex:
        return "ERR " + str(ex)[:120]


# 1. triangle: fixed base on origin (horizontal 30), sides a,b by dimensions
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
A = sk.originPoint
l1 = L.addByTwoPoints(A, P3(mm(30), 0, 0))
l2 = L.addByTwoPoints(l1.endSketchPoint, P3(mm(15), mm(20), 0))
l3 = L.addByTwoPoints(l2.endSketchPoint, l1.startSketchPoint)
G = sk.geometricConstraints
D = sk.sketchDimensions
G.addHorizontal(l1)
d1 = D.addDistanceDimension(l1.startSketchPoint, l1.endSketchPoint, 0, P3(mm(15), mm(-5), 0))
d2 = D.addDistanceDimension(l2.startSketchPoint, l2.endSketchPoint, 0, P3(mm(30), mm(15), 0))
d3 = D.addDistanceDimension(l3.startSketchPoint, l3.endSketchPoint, 0, P3(0, mm(15), 0))
setv(d1, "30 mm"); setv(d2, "25 mm"); setv(d3, "25 mm")
apex = l2.endSketchPoint
tri = {"fully": sk.isFullyConstrained, "apex0": pt(apex)}
tri["impossible_10_10"] = [setv(d2, "10 mm"), setv(d3, "10 mm"), pt(apex), d2.parameter.expression, d3.parameter.expression]
tri["restore"] = [setv(d2, "25 mm"), setv(d3, "25 mm"), pt(apex)]
tri["degenerate_15_15"] = [setv(d2, "15 mm"), setv(d3, "15 mm"), pt(apex)]
tri["back_25"] = [setv(d2, "25 mm"), setv(d3, "25 mm"), pt(apex)]
apexes = set()
for i in range(10):
    setv(d2, "20 mm" if i % 2 == 0 else "28 mm")
    apexes.add(tuple(pt(apex)))
tri["branch_alternating_distinct"] = len(apexes)
tri["branch_signs_y"] = sorted({1 if a[1] > 0 else -1 for a in apexes})
out["triangle"] = tri

# 2. tangency chain: line - arc - line, tangent at both joints, radius driven
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
a = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
arc = sk.sketchCurves.sketchArcs.addByCenterStartSweep(P3(mm(20), mm(10), 0), a.endSketchPoint, 3.14159265 / 2)
b = L.addByTwoPoints(arc.endSketchPoint, P3(mm(30), mm(30), 0))
G = sk.geometricConstraints
G.addTangent(a, arc)
G.addTangent(arc, b)
rd = sk.sketchDimensions.addRadialDimension(arc, P3(mm(25), mm(15), 0))
chain = {"r0": round(arc.radius * 10, 4)}
res = []
for r in ("5 mm", "15 mm", "2 mm", "40 mm", "10 mm"):
    s = setv(rd, r)
    # tangency check: angle between line a and arc tangent at joint
    res.append([r, s, round(arc.radius * 10, 4), pt(a.endSketchPoint), pt(b.endSketchPoint)])
chain["edits"] = res
chain["cons"] = sk.geometricConstraints.count
out["tangent_chain"] = chain

# 3. near-degenerate constraints
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
G = sk.geometricConstraints
nd = {}
p = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
q = L.addByTwoPoints(P3(0, mm(5), 0), P3(mm(20), mm(5.00001), 0))
try:
    G.addParallel(p, q); nd["parallel_almost"] = "ok"
except Exception as ex:
    nd["parallel_almost"] = str(ex)[:120]
r_ = L.addByTwoPoints(P3(0, mm(10), 0), P3(mm(20), mm(10), 0))
try:
    G.addParallel(p, r_); nd["parallel_exact_already"] = "ok"
except Exception as ex:
    nd["parallel_exact_already"] = str(ex)[:120]
s_ = L.addByTwoPoints(P3(0, mm(20), 0), P3(mm(1e-4), mm(20), 0))
try:
    G.addHorizontal(s_); nd["horizontal_on_1e-4mm_line"] = "ok"
except Exception as ex:
    nd["horizontal_on_1e-4mm_line"] = str(ex)[:120]
t_ = L.addByTwoPoints(P3(mm(40), 0, 0), P3(mm(40), mm(20), 0))
try:
    G.addCollinear(p, t_); nd["collinear_perpendicular"] = "ok"
except Exception as ex:
    nd["collinear_perpendicular"] = str(ex)[:120]
nd["t_after"] = [pt(t_.startSketchPoint), pt(t_.endSketchPoint)]
out["near_degenerate"] = nd
result.update(out)
result["doc"] = doc.name
