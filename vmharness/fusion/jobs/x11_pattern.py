# MKProbe job (X-11): sketch circular/rectangular pattern, move, copy (API). Record created curves/constraints/dims,
# and whether editing the pattern source propagates to the copies.
P3 = adsk.core.Point3D.create
VS = adsk.core.ValueInput.createByString

def close_unsaved():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)

def snap(sk):
    cs = []
    for i in range(sk.sketchCurves.count):
        c = sk.sketchCurves.item(i)
        if c.objectType.endswith("SketchCircle"):
            cs.append(["C", round(c.centerSketchPoint.geometry.x * 10, 2), round(c.centerSketchPoint.geometry.y * 10, 2), round(c.radius * 10, 3), c.isConstruction])
        elif c.objectType.endswith("SketchLine"):
            cs.append(["L", round(c.startSketchPoint.geometry.x * 10, 2), round(c.startSketchPoint.geometry.y * 10, 2), round(c.endSketchPoint.geometry.x * 10, 2), round(c.endSketchPoint.geometry.y * 10, 2), c.isConstruction])
    return {"curves": cs, "cons": [sk.geometricConstraints.item(i).objectType.split("::")[-1] for i in range(sk.geometricConstraints.count)],
            "dims": [sk.sketchDimensions.item(i).parameter.expression for i in range(sk.sketchDimensions.count)],
            "profiles": sk.profiles.count}

def coll(*xs):
    c = adsk.core.ObjectCollection.create()
    for x in xs:
        c.add(x)
    return c

close_unsaved()
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
result["gc_create"] = [m for m in dir(root.sketches.add(root.xYConstructionPlane).geometricConstraints) if m.startswith("create")]

# circular pattern
sk = root.sketches.add(root.xYConstructionPlane)
c = sk.sketchCurves.sketchCircles.addByCenterRadius(P3(3, 0, 0), 0.5)
r = {}
try:
    gc = sk.geometricConstraints
    inp = gc.createCircularPatternInput([c], sk.originPoint)
    inp.quantity = VS("6")
    pat = gc.addCircularPattern(inp)
    r["after"] = snap(sk)
    r["pattern_props"] = [m for m in dir(pat) if not m.startswith("_")][:60]
    sk.sketchDimensions.addDiameterDimension(c, P3(4, 1, 0)).parameter.expression = "20 mm"
    r["after_src_edit"] = snap(sk)
    try:
        pat.quantity.expression = "3"
        r["after_qty3"] = snap(sk)
    except Exception as ex:
        r["qty_err"] = str(ex)[:200]
except Exception as ex:
    r["error"] = str(ex)[:300]
result["circular"] = r

# rectangular pattern
sk = root.sketches.add(root.xYConstructionPlane)
c = sk.sketchCurves.sketchCircles.addByCenterRadius(P3(0, 0, 0), 0.3)
r = {}
try:
    gc = sk.geometricConstraints
    inp = gc.createRectangularPatternInput([c], adsk.fusion.PatternDistanceType.SpacingPatternDistanceType)
    inp.setDirectionOne(sk.sketchCurves.sketchLines.addByTwoPoints(P3(0, -1, 0), P3(1, -1, 0)), VS("3"), VS("10 mm"))
    inp.setDirectionTwo(sk.sketchCurves.sketchLines.addByTwoPoints(P3(-1, 0, 0), P3(-1, 1, 0)), VS("2"), VS("10 mm"))
    gc.addRectangularPattern(inp)
    r["after"] = snap(sk)
except Exception as ex:
    r["error"] = str(ex)[:300]
    r["state"] = snap(sk)
result["rect"] = r

# move and copy
sk = root.sketches.add(root.xYConstructionPlane)
l = sk.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(2, 0, 0))
sk.geometricConstraints.addHorizontal(l)
m = adsk.core.Matrix3D.create()
m.translation = adsk.core.Vector3D.create(1, 1, 0)
r = {}
try:
    sk.move(coll(l), m)
    r["moved"] = snap(sk)
    rot = adsk.core.Matrix3D.create()
    rot.setToRotation(0.5236, adsk.core.Vector3D.create(0, 0, 1), P3(0, 0, 0))
    sk.copy(coll(l), rot)
    r["copied_rot30"] = snap(sk)
    sk.move(coll(l), rot)
    r["moved_rot30_horizontal_line"] = snap(sk)
except Exception as ex:
    r["error"] = str(ex)[:300]
    r["state"] = snap(sk)
result["move_copy"] = r
close_unsaved()
