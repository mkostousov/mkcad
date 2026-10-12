# MKProbe job: refinement 3 (dimension formulas referencing params and other dims) + 4 (remaining construction types).
import math
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
H = adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation
Vt = adsk.fusion.DimensionOrientations.VerticalDimensionOrientation
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
up = design.userParameters
up.add("Len", VI("40 mm"), "mm", "")
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
a = L.addByTwoPoints(sk.originPoint, P3(4, 0, 0)); b = L.addByTwoPoints(a.endSketchPoint, P3(4, 2, 0))
c = L.addByTwoPoints(b.endSketchPoint, P3(0, 2, 0)); dd = L.addByTwoPoints(c.endSketchPoint, a.startSketchPoint)
g = sk.geometricConstraints; g.addHorizontal(a); g.addHorizontal(c); g.addVertical(b); g.addVertical(dd)
w = sk.sketchDimensions.addDistanceDimension(a.startSketchPoint, a.endSketchPoint, H, P3(2, -1, 0))
h = sk.sketchDimensions.addDistanceDimension(b.startSketchPoint, b.endSketchPoint, Vt, P3(5, 1, 0))
rows = []


def setx(param, expr, label):
    try:
        param.expression = expr
        rows.append({"case": label, "ok": True, "expr": param.expression, "value_mm": round(param.value * 10, 6),
                     "w_mm": round(w.parameter.value * 10, 6), "h_mm": round(h.parameter.value * 10, 6)})
    except Exception as e:
        rows.append({"case": label, "ok": False, "error": str(e).strip()[:160], "expr_after": param.expression})


rows.append({"names": {"w": w.parameter.name, "h": h.parameter.name}})
setx(w.parameter, "Len / 2 + 5 mm", "w = Len/2 + 5 mm")
setx(h.parameter, w.parameter.name + " * 2", f"h = {w.parameter.name} * 2 (dimension referencing a dimension)")
setx(up.itemByName("Len"), "60 mm", "Len -> 60 mm (chain Len -> w -> h)")
setx(w.parameter, h.parameter.name + " / 2", f"w = {h.parameter.name}/2 (cycle w<->h)")
setx(h.parameter, "Len * 1", "h = Len * 1 (unit check)")
setx(h.parameter, "Len * 1 mm", "h = Len * 1 mm (length*length — unit error expected)")
try:
    uses = [p.name for p in up.itemByName("Len").dependentParameters]
except Exception as e:
    uses = str(e)[:80]
rows.append({"Len.dependentParameters": uses})
result["formulas"] = rows

# construction types
ext = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
body = ext.bodies.item(0)
F = lambda pred: next(body.faces.item(i) for i in range(body.faces.count) if pred(body.faces.item(i).centroid))
E = lambda pred: next(body.edges.item(i) for i in range(body.edges.count) if pred(body.edges.item(i)))
top = F(lambda c: abs(c.z - 1) < 1e-6)
e_top_front = E(lambda e: all(abs(v.geometry.y) < 1e-6 and abs(v.geometry.z - 1) < 1e-6 for v in (e.startVertex, e.endVertex)))
e_bot_back = E(lambda e: all(abs(v.geometry.y - (w.parameter.value / 10 * 0 + b.endSketchPoint.geometry.y)) < 1e-6 and abs(v.geometry.z) < 1e-6
                             for v in (e.startVertex, e.endVertex)))
cyl_sk = root.sketches.add(root.xYConstructionPlane)
cyl_sk.sketchCurves.sketchCircles.addByCenterRadius(P3(-3, 0, 0), 1)
cyl = root.features.extrudeFeatures.addSimple(cyl_sk.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation).bodies.item(0)
cyl_face = next(cyl.faces.item(i) for i in range(cyl.faces.count) if cyl.faces.item(i).geometry.objectType.endswith("Cylinder"))
v = [body.vertices.item(i) for i in range(3)]
cp, ca, cpt = root.constructionPlanes, root.constructionAxes, root.constructionPoints
cons = []


def try_(label, fn):
    try:
        o = fn()
        geo = o.geometry
        info = {"case": label, "ok": True, "tl": o.timelineObject.name if o.timelineObject else None}
        if hasattr(geo, "normal"):
            info["normal"] = [round(geo.normal.x, 3), round(geo.normal.y, 3), round(geo.normal.z, 3)]
        if hasattr(geo, "direction"):
            info["dir"] = [round(geo.direction.x, 3), round(geo.direction.y, 3), round(geo.direction.z, 3)]
        if hasattr(geo, "x") and hasattr(geo, "z") and not hasattr(geo, "normal"):
            info["at_mm"] = [round(geo.x * 10, 3), round(geo.y * 10, 3), round(geo.z * 10, 3)]
        cons.append(info)
    except Exception as e:
        cons.append({"case": label, "ok": False, "error": str(e).strip()[:140]})


def P(setter):
    def f():
        i = cp.createInput(); setter(i); return cp.add(i)
    return f


def A(setter):
    def f():
        i = ca.createInput(); setter(i); return ca.add(i)
    return f


def Pt(setter):
    def f():
        i = cpt.createInput(); setter(i); return cpt.add(i)
    return f


try_("plane tangent to cylinder face", P(lambda i: i.setByTangent(cyl_face, VI("0 deg"), root.xYConstructionPlane)))
try_("plane through two edges", P(lambda i: i.setByTwoEdges(e_top_front, e_bot_back)))
try_("plane through three points", P(lambda i: i.setByThreePoints(v[0], v[1], v[2])))
try_("plane tangent at point", P(lambda i: i.setByTangentAtPoint(cyl_face, cyl.vertices.item(0))))
try_("plane along path (edge, 50%)", P(lambda i: i.setByDistanceOnPath(e_top_front, VI("0.5"))))
try_("axis through cylinder", A(lambda i: i.setByCircularFace(cyl_face)))
try_("axis perpendicular at point", A(lambda i: i.setByPerpendicularAtPoint(top, body.vertices.item(0))))
try_("axis two planes", A(lambda i: i.setByTwoPlanes(root.xZConstructionPlane, root.yZConstructionPlane)))
try_("axis through edge", A(lambda i: i.setByEdge(e_top_front)))
try_("axis normal to face at point", A(lambda i: i.setByNormalToFaceAtPoint(top, body.vertices.item(0))))
try_("point at vertex", Pt(lambda i: i.setByPoint(body.vertices.item(0))))
try_("point two edges", Pt(lambda i: i.setByTwoEdges(e_top_front, F(lambda c: abs(c.x) < 1e-6).edges.item(0))))
try_("point three planes", Pt(lambda i: i.setByThreePlanes(root.xYConstructionPlane, root.xZConstructionPlane, root.yZConstructionPlane)))
try_("point centre of circle", Pt(lambda i: i.setByCenter(cyl_sk.sketchCurves.sketchCircles.item(0))))
try_("point edge and plane", Pt(lambda i: i.setByEdgePlane(e_top_front, root.yZConstructionPlane)))
result["construct"] = cons
