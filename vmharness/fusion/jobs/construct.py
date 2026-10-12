# MKProbe job: construction planes/axes on a 40x20x10 box (spec A.3 Construction, M).
import math
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
s = root.sketches.add(root.xYConstructionPlane)
s.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
body = root.features.extrudeFeatures.addSimple(s.profiles.item(0), VI("10 mm"),
                                               adsk.fusion.FeatureOperations.NewBodyFeatureOperation).bodies.item(0)
F = lambda pred: next(body.faces.item(i) for i in range(body.faces.count) if pred(body.faces.item(i).centroid))
top, bottom = F(lambda c: abs(c.z - 1) < 1e-6), F(lambda c: abs(c.z) < 1e-6)
left, front = F(lambda c: abs(c.x) < 1e-6), F(lambda c: abs(c.y) < 1e-6)
edge_top_front = next(body.edges.item(i) for i in range(body.edges.count)
                      if all(abs(v.geometry.y) < 1e-6 and abs(v.geometry.z - 1) < 1e-6
                             for v in (body.edges.item(i).startVertex, body.edges.item(i).endVertex)))
cp = root.constructionPlanes
rows = []


def plane_info(label, make):
    try:
        p = make()
        g = p.geometry
        n, o = g.normal, g.origin
        rows.append({"case": label, "ok": True, "normal": [round(n.x, 4), round(n.y, 4), round(n.z, 4)],
                     "origin_mm": [round(o.x * 10, 3), round(o.y * 10, 3), round(o.z * 10, 3)],
                     "tl": p.timelineObject.name if p.timelineObject else None})
    except Exception as e:
        rows.append({"case": label, "ok": False, "error": str(e).strip()[:160]})


def offset(ent, v):
    i = cp.createInput(); i.setByOffset(ent, VI(v)); return cp.add(i)


def angle(edge, v, ref):
    i = cp.createInput(); i.setByAngle(edge, VI(v), ref); return cp.add(i)


def mid(a, b):
    i = cp.createInput(); i.setByTwoPlanes(a, b); return cp.add(i)


plane_info("offset 5 mm from top face", lambda: offset(top, "5 mm"))
plane_info("offset -5 mm from top face", lambda: offset(top, "-5 mm"))
plane_info("plane at 30 deg about top-front edge, ref top face", lambda: angle(edge_top_front, "30 deg", top))
plane_info("midplane top/bottom (parallel)", lambda: mid(top, bottom))
plane_info("midplane left/front (90 deg)", lambda: mid(left, front))
ca = root.constructionAxes
try:
    i = ca.createInput()
    i.setByTwoPoints(body.vertices.item(0), body.vertices.item(1))
    a = ca.add(i)
    g = a.geometry
    rows.append({"case": "axis through two vertices", "ok": True,
                 "dir": [round(g.direction.x, 4), round(g.direction.y, 4), round(g.direction.z, 4)]})
except Exception as e:
    rows.append({"case": "axis through two vertices", "ok": False, "error": str(e)[:160]})
result["rows"] = rows
result["timeline"] = [design.timeline.item(i).name for i in range(design.timeline.count)]
