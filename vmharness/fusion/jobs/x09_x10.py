# MKProbe job (X-09, X-10).
# X-09: angular dimension between two lines sharing an endpoint: stored value/expression, which line moves when the
#       value is edited (30 -> 60 -> 120 -> 200 -> -10 deg), and acceptance of 0 / 180.
# X-10: sketch fillet radius larger than the lines allow (F83): resulting curves.
P3 = adsk.core.Point3D.create
VS = adsk.core.ValueInput.createByString

def close_unsaved():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)

def mm(p):
    return [round(p.x * 10, 3), round(p.y * 10, 3)]

def curves(sk):
    out = []
    for i in range(sk.sketchCurves.count):
        c = sk.sketchCurves.item(i)
        t = c.objectType.split("::")[-1]
        if t == "SketchLine":
            out.append(["L", mm(c.startSketchPoint.geometry), mm(c.endSketchPoint.geometry)])
        elif t == "SketchArc":
            out.append(["A", mm(c.centerSketchPoint.geometry), round(c.radius * 10, 3), mm(c.startSketchPoint.geometry), mm(c.endSketchPoint.geometry)])
        else:
            out.append([t])
    return out

close_unsaved()
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
root = adsk.fusion.Design.cast(app.activeProduct).rootComponent

# X-09 angle
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
a = L.addByTwoPoints(P3(0, 0, 0), P3(3, 0, 0))
b = L.addByTwoPoints(a.startSketchPoint, P3(2, 2, 0))
a.startSketchPoint.isFixed = True
sk.geometricConstraints.addHorizontal(a)
r = {"before": curves(sk)}
d = sk.sketchDimensions.addAngularDimension(a, b, P3(1.5, 0.5, 0))
r["created"] = [d.parameter.expression, round(d.parameter.value, 6), d.isDriving]
seq = []
for v in ("30 deg", "60 deg", "120 deg", "200 deg", "-10 deg", "0 deg", "180 deg", "45 deg"):
    try:
        d.parameter.expression = v
        seq.append([v, "ok", d.parameter.expression, curves(sk)])
    except Exception as ex:
        seq.append([v, "ERR " + str(ex)[:120], d.parameter.expression, curves(sk)])
r["seq"] = seq
result["x09_angle"] = r

# X-10 oversize fillet: two 20 mm lines at 90 deg, fillet r = 5, 20, 50 mm (fresh sketch each)
for rad in (0.5, 2.0, 5.0):
    sk = root.sketches.add(root.xYConstructionPlane)
    L = sk.sketchCurves.sketchLines
    l1 = L.addByTwoPoints(P3(0, 0, 0), P3(2, 0, 0))
    l2 = L.addByTwoPoints(l1.endSketchPoint, P3(2, 2, 0))
    try:
        arc = sk.sketchCurves.sketchArcs.addFillet(l1, l1.endSketchPoint.geometry, l2, l2.endSketchPoint.geometry, rad)
        result["x10_r%g" % (rad * 10)] = {"curves": curves(sk), "dims": [sk.sketchDimensions.item(i).parameter.expression for i in range(sk.sketchDimensions.count)],
                                         "cons": [sk.geometricConstraints.item(i).objectType.split("::")[-1] for i in range(sk.geometricConstraints.count)]}
    except Exception as ex:
        result["x10_r%g" % (rad * 10)] = {"error": str(ex)[:200], "curves": curves(sk)}
close_unsaved()
