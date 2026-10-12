# MKProbe job: solver probe on a fully constrained rectangle (redundancy, conflict, determinism, driven dims).
P3 = adsk.core.Point3D.create
H = adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation
Vt = adsk.fusion.DimensionOrientations.VerticalDimensionOrientation
steps = []


def r(x, n=9):
    return round(x, n)


def state(sk, label, **extra):
    pts = []
    for i in range(sk.sketchCurves.sketchLines.count):
        l = sk.sketchCurves.sketchLines.item(i)
        pts.append([r(l.startSketchPoint.geometry.x, 6), r(l.startSketchPoint.geometry.y, 6)])
    cons = {}
    for i in range(sk.geometricConstraints.count):
        t = sk.geometricConstraints.item(i).objectType.split("::")[-1].replace("Constraint", "")
        cons[t] = cons.get(t, 0) + 1
    dims = [(d.parameter.expression if d.parameter else "?") + ("" if d.isDriving else "(driven)")
            for d in [sk.sketchDimensions.item(i) for i in range(sk.sketchDimensions.count)]]
    s = {"step": label, "fully": sk.isFullyConstrained,
         "curvesFully": sum(1 for i in range(sk.sketchCurves.count) if sk.sketchCurves.item(i).isFullyConstrained),
         "curves": sk.sketchCurves.count, "profiles": sk.profiles.count, "cons": cons, "dims": dims, "starts": pts}
    s.update(extra)
    steps.append(s)


def attempt(fn):
    try:
        out = fn()
        d = {"ok": True}
        if out is not None and hasattr(out, "isDriving"):
            d["isDriving"] = out.isDriving
        return d
    except Exception as e:
        return {"ok": False, "error": str(e).strip()[:300]}


app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
gc = sk.geometricConstraints
a = L.addByTwoPoints(sk.originPoint, P3(4, 0, 0))           # bottom, starts AT origin point
b = L.addByTwoPoints(a.endSketchPoint, P3(4, 2, 0))         # right
c = L.addByTwoPoints(b.endSketchPoint, P3(0, 2, 0))         # top
d = L.addByTwoPoints(c.endSketchPoint, a.startSketchPoint)  # left, closes loop
state(sk, "4 connected lines, first starts on origin point")
gc.addHorizontal(a); gc.addHorizontal(c); gc.addVertical(b); gc.addVertical(d)
state(sk, "H/V constraints added")
dims = sk.sketchDimensions
state(sk, "width dim", res=attempt(lambda: dims.addDistanceDimension(a.startSketchPoint, a.endSketchPoint, H, P3(2, -1, 0))))
state(sk, "height dim", res=attempt(lambda: dims.addDistanceDimension(b.startSketchPoint, b.endSketchPoint, Vt, P3(5, 1, 0))))
state(sk, "redundant width on top edge", res=attempt(lambda: dims.addDistanceDimension(c.startSketchPoint, c.endSketchPoint, H, P3(2, 3, 0))))
state(sk, "redundant geometric: parallel a||c", res=attempt(lambda: gc.addParallel(a, c)))
state(sk, "conflicting geometric: perpendicular a,c", res=attempt(lambda: gc.addPerpendicular(a, c)))
wd = dims.item(0)
before = wd.parameter.expression
state(sk, "conflicting value: width = 0", res=attempt(lambda: setattr(wd.parameter, "expression", "0 mm")),
      width_after=wd.parameter.expression, width_before=before)
state(sk, "negative value: width = -10 mm", res=attempt(lambda: setattr(wd.parameter, "expression", "-10 mm")),
      width_after=wd.parameter.expression)
results = set()
for i in range(40):
    wd.parameter.expression = "73.3 mm" if i % 2 == 0 else "40 mm"
    if i % 2 == 1:
        results.add(tuple((r(l.startSketchPoint.geometry.x), r(l.startSketchPoint.geometry.y))
                          for l in (a, b, c, d)))
state(sk, "20 width round-trips 40<->73.3 mm", distinct_results=len(results))
# Fully constrained drag attempt: move a point via API
p = b.endSketchPoint
mv = attempt(lambda: p.move(adsk.core.Vector3D.create(1, 1, 0)))
state(sk, "move fully-constrained corner by (10,10)mm", res=mv)
# Unit expressions
state(sk, "expression in inches", res=attempt(lambda: setattr(wd.parameter, "expression", "2 in")),
      width_value_cm=r(wd.parameter.value, 6))
state(sk, "expression with user parameter", res=attempt(lambda: (
    design.userParameters.add("W", adsk.core.ValueInput.createByString("35 mm"), "mm", ""),
    setattr(wd.parameter, "expression", "W * 2"))), width_value_cm=r(wd.parameter.value, 6))
result["steps"] = steps
