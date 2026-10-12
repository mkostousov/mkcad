# MKProbe job: MeasureManager readouts on known geometry (spec A.3 Inspect, M).
import math
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
FO = adsk.fusion.FeatureOperations


def box(x0, y0, x1, y1, h):
    s = root.sketches.add(root.xYConstructionPlane)
    s.sketchCurves.sketchLines.addTwoPointRectangle(P3(x0, y0, 0), P3(x1, y1, 0))
    return root.features.extrudeFeatures.addSimple(s.profiles.item(0), VI(h), FO.NewBodyFeatureOperation).bodies.item(0)


a = box(0, 0, 2, 2, "10 mm")        # 20x20x10 at origin
b = box(5, 0, 7, 2, "10 mm")        # 30 mm gap in x
mm = app.measureManager
face = lambda body, pred: next(body.faces.item(i) for i in range(body.faces.count) if pred(body.faces.item(i).centroid))
a_right = face(a, lambda c: abs(c.x - 2) < 1e-6)
b_left = face(b, lambda c: abs(c.x - 5) < 1e-6)
a_top = face(a, lambda c: abs(c.z - 1) < 1e-6)
rows = []
r = mm.measureMinimumDistance(a_right, b_left)
rows.append({"what": "min distance parallel faces 30 mm apart", "value_mm": round(r.value * 10, 6)})
r = mm.measureMinimumDistance(a, b)
rows.append({"what": "min distance body-body", "value_mm": round(r.value * 10, 6)})
r = mm.measureMinimumDistance(a.vertices.item(0), b)
rows.append({"what": "min distance vertex-body", "value_mm": round(r.value * 10, 6)})
r = mm.measureAngle(a_right, a_top)
rows.append({"what": "angle right face vs top face", "value_deg": round(math.degrees(r.value), 6)})
r = mm.measureAngle(a_right, b_left)
rows.append({"what": "angle between parallel faces", "value_deg": round(math.degrees(r.value), 6)})
try:
    ob = mm.getOrientedBoundingBox(a, adsk.core.Vector3D.create(1, 0, 0), adsk.core.Vector3D.create(0, 1, 0))
    rows.append({"what": "oriented bbox of a", "lwh_mm": [round(ob.length * 10, 3), round(ob.width * 10, 3), round(ob.height * 10, 3)]})
except Exception as e:
    rows.append({"what": "oriented bbox", "error": str(e)[:120]})
result["rows"] = rows
