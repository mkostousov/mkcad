# MKProbe job: report active sketch curves (mm), constraints with their entities, and current model->view mapping.
P3 = adsk.core.Point3D.create
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.item(root.sketches.count - 1)
vp = app.activeViewport


def r(v):
    return round(v * 10, 3)


cs = []
for i in range(sk.sketchCurves.count):
    c = sk.sketchCurves.item(i)
    e = {"i": i, "t": c.objectType.split("::")[-1].replace("Sketch", "")}
    try:
        e["s"] = [r(c.startSketchPoint.geometry.x), r(c.startSketchPoint.geometry.y)]
        e["e"] = [r(c.endSketchPoint.geometry.x), r(c.endSketchPoint.geometry.y)]
        e["sh"] = [c.startSketchPoint.connectedEntities.count, c.endSketchPoint.connectedEntities.count]
    except Exception:
        e["c"] = [r(c.centerSketchPoint.geometry.x), r(c.centerSketchPoint.geometry.y)]
    cs.append(e)
gc = []
for i in range(sk.geometricConstraints.count):
    g = sk.geometricConstraints.item(i)
    gc.append(g.objectType.split("::")[-1].replace("Constraint", ""))
result["curves"] = cs
result["cons"] = gc
result["dims"] = sk.sketchDimensions.count
result["active"] = app.activeEditObject.objectType.split("::")[-1]
m = {}
for name, x, y in (("o", 0, 0), ("x10", 1, 0)):
    p = vp.modelToViewSpace(P3(x, y, 0))
    m[name] = [round(p.x, 1), round(p.y, 1)]
result["map"] = m
