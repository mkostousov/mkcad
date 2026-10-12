# MKProbe job: sketch mirror (via Symmetry), sketch scale, curvature (Smooth) constraint — closes F.7/F.3 [U] rows.
P3 = adsk.core.Point3D.create
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
rows = []


def summary(sk):
    cs = {}
    for i in range(sk.geometricConstraints.count):
        t = sk.geometricConstraints.item(i).objectType.split("::")[-1].replace("Constraint", "")
        cs[t] = cs.get(t, 0) + 1
    return {"curves": sk.sketchCurves.count, "constraints": cs, "dims": sk.sketchDimensions.count}


# Scale: rectangle with a 40 mm dimension, scale x2 about origin
sk = root.sketches.add(root.xYConstructionPlane)
ls = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
dim = sk.sketchDimensions.addDistanceDimension(ls.item(0).startSketchPoint, ls.item(0).endSketchPoint,
                                               adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0))
ents = adsk.core.ObjectCollection.create()
for i in range(sk.sketchCurves.count):
    ents.add(sk.sketchCurves.item(i))
try:
    m = adsk.core.Matrix3D.create()
    m.setCell(0, 0, 2); m.setCell(1, 1, 2)
    sk.move(ents, m)
    rows.append({"case": "scale x2 via move(matrix) with a driving 40 mm dim", **summary(sk),
                 "dim_expr": dim.parameter.expression, "line0_mm": round(ls.item(0).length * 10, 3)})
except Exception as e:
    rows.append({"case": "scale x2 via move(matrix)", "error": str(e)[:160], "line0_mm": round(ls.item(0).length * 10, 3)})
# Curvature (Smooth) constraint between a spline and an arc
sk2 = root.sketches.add(root.xYConstructionPlane)
arc = sk2.sketchCurves.sketchArcs.addByThreePoints(P3(0, 0, 0), P3(1, 1, 0), P3(2, 0, 0))
pts = adsk.core.ObjectCollection.create()
for p in (P3(2, 0, 0), P3(3, -1, 0), P3(4, 0, 0)):
    pts.add(p)
spl = sk2.sketchCurves.sketchFittedSplines.add(pts)
try:
    sk2.geometricConstraints.addCoincident(spl.startSketchPoint, arc.endSketchPoint)
    sk2.geometricConstraints.addSmooth(spl, arc)
    rows.append({"case": "Smooth (curvature) spline-arc", "ok": True, **summary(sk2)})
except Exception as e:
    rows.append({"case": "Smooth (curvature) spline-arc", "ok": False, "error": str(e)[:160], **summary(sk2)})
result["rows"] = rows
result["mirror_note"] = "no Sketch.mirror API; sketch Mirror command is GUI (MirrorSketchCommand) — creates Symmetry constraints per reference docs [U]"
