# MKProbe job: read the most recent sketch of the active design (curves, constraints, dimensions, state).
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.item(root.sketches.count - 1)
curves = []
for i in range(sk.sketchCurves.count):
    c = sk.sketchCurves.item(i)
    e = {"type": c.objectType.split("::")[-1], "construction": c.isConstruction, "fully": c.isFullyConstrained}
    try:
        e["start"] = [round(c.startSketchPoint.geometry.x * 10, 3), round(c.startSketchPoint.geometry.y * 10, 3)]
        e["end"] = [round(c.endSketchPoint.geometry.x * 10, 3), round(c.endSketchPoint.geometry.y * 10, 3)]
    except Exception:
        pass
    curves.append(e)
cons = []
for i in range(sk.geometricConstraints.count):
    g = sk.geometricConstraints.item(i)
    cons.append(g.objectType.split("::")[-1].replace("Constraint", ""))
dims = []
for i in range(sk.sketchDimensions.count):
    d = sk.sketchDimensions.item(i)
    dims.append({"type": d.objectType.split("::")[-1], "expr": d.parameter.expression if d.parameter else None,
                 "driving": d.isDriving})
result["sketch"] = {"name": sk.name, "isActive": app.activeEditObject == sk, "fully": sk.isFullyConstrained,
                    "profiles": sk.profiles.count, "curves": curves, "constraints": cons, "dims": dims,
                    "points": sk.sketchPoints.count}
result["units_note"] = "coordinates in mm"
