# MKProbe job: closure tolerance bisection + sketch editing ops via API (trim, extend, split/break, offset, fillet,
# move/scale via transform, projected geometry when the source is deleted). New untitled design.
P3 = adsk.core.Point3D.create
doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
out = {}


def mm(v):
    return v / 10.0


def r3(v):
    return round(v * 10, 4)


def curves(sk):
    res = []
    for i in range(sk.sketchCurves.count):
        c = sk.sketchCurves.item(i)
        e = {"t": c.objectType.split("::")[-1].replace("Sketch", "")}
        try:
            e["s"] = [r3(c.startSketchPoint.geometry.x), r3(c.startSketchPoint.geometry.y)]
            e["e"] = [r3(c.endSketchPoint.geometry.x), r3(c.endSketchPoint.geometry.y)]
        except Exception:
            pass
        try:
            e["r"] = r3(c.radius)
            e["c"] = [r3(c.centerSketchPoint.geometry.x), r3(c.centerSketchPoint.geometry.y)]
        except Exception:
            pass
        res.append(e)
    return res


def cons(sk):
    return [sk.geometricConstraints.item(i).objectType.split("::")[-1].replace("Constraint", "")
            for i in range(sk.geometricConstraints.count)]


def try_(name, f):
    try:
        out[name] = f()
    except Exception as ex:
        out[name] = "ERR " + str(ex)[:200]


# 1. closure tolerance bisection
gaps = {}
for g in (2e-6, 5e-6, 1e-5, 2e-5, 5e-5):
    sk = root.sketches.add(root.xYConstructionPlane)
    L = sk.sketchCurves.sketchLines
    a = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
    b = L.addByTwoPoints(a.endSketchPoint, P3(mm(20), mm(20), 0))
    c = L.addByTwoPoints(b.endSketchPoint, P3(0, mm(20), 0))
    L.addByTwoPoints(c.endSketchPoint, P3(0, mm(g), 0))
    gaps["%g" % g] = sk.profiles.count
    sk.deleteMe()
out["gap_profiles"] = gaps

# 2. trim: two crossing lines, trim one segment near (5,0)
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "trim"
L = sk.sketchCurves.sketchLines
h = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
v = L.addByTwoPoints(P3(mm(10), mm(-10), 0), P3(mm(10), mm(10), 0))
try_("trim_ret", lambda: h.trim(P3(mm(5), 0, 0)).count)
out["trim_after"] = curves(sk)
out["trim_cons"] = cons(sk)

# 3. trim middle of a line crossed by two lines -> split into two pieces?
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "trim_mid"
L = sk.sketchCurves.sketchLines
h = L.addByTwoPoints(P3(0, 0, 0), P3(mm(30), 0, 0))
L.addByTwoPoints(P3(mm(10), mm(-5), 0), P3(mm(10), mm(5), 0))
L.addByTwoPoints(P3(mm(20), mm(-5), 0), P3(mm(20), mm(5), 0))
try_("trim_mid_ret", lambda: h.trim(P3(mm(15), 0, 0)).count)
out["trim_mid_after"] = curves(sk)
out["trim_mid_cons"] = cons(sk)

# 4. trim an isolated line (no intersections) -> deleted?
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "trim_iso"
iso = sk.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(mm(10), 0, 0))
try_("trim_iso_ret", lambda: iso.trim(P3(mm(5), 0, 0)).count)
out["trim_iso_after"] = curves(sk)

# 5. extend: short line toward a vertical boundary
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "extend"
L = sk.sketchCurves.sketchLines
s = L.addByTwoPoints(P3(0, 0, 0), P3(mm(5), 0, 0))
L.addByTwoPoints(P3(mm(20), mm(-10), 0), P3(mm(20), mm(10), 0))
try_("extend_ret", lambda: s.extend(P3(mm(4.9), 0, 0)).count)
out["extend_after"] = curves(sk)
out["extend_cons"] = cons(sk)

# 6. break (split) at intersection
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "break"
L = sk.sketchCurves.sketchLines
h = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
L.addByTwoPoints(P3(mm(8), mm(-5), 0), P3(mm(8), mm(5), 0))
try_("break_ret", lambda: h.breakCurve(P3(mm(3), 0, 0)).count)
out["break_after"] = curves(sk)
out["break_cons"] = cons(sk)

# 7. offset of a rectangle chain, both directions
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "offset"
lines = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(mm(20), mm(10), 0))
coll = adsk.core.ObjectCollection.create()
coll.add(lines.item(0))
chain = sk.findConnectedCurves(lines.item(0))
out["connected_found"] = chain.count
try_("offset_out", lambda: sk.offset(chain, P3(mm(-5), mm(-5), 0), mm(2)).count)
try_("offset_in", lambda: sk.offset(chain, P3(mm(10), mm(5), 0), mm(2)).count)
try_("offset_in_too_big", lambda: sk.offset(chain, P3(mm(10), mm(5), 0), mm(6)).count)
out["offset_curves"] = sk.sketchCurves.count
out["offset_cons"] = cons(sk)
out["offset_dims"] = [sk.sketchDimensions.item(i).objectType.split("::")[-1] + ":" +
                      (sk.sketchDimensions.item(i).parameter.expression if sk.sketchDimensions.item(i).parameter else "")
                      for i in range(sk.sketchDimensions.count)]
out["offset_profiles"] = sk.profiles.count

# 8. sketch fillet at a corner
sk = root.sketches.add(root.xYConstructionPlane)
sk.name = "fillet"
L = sk.sketchCurves.sketchLines
a = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
b = L.addByTwoPoints(a.endSketchPoint, P3(mm(20), mm(20), 0))
try_("fillet_ret", lambda: sk.sketchCurves.sketchArcs.addFillet(a, a.endSketchPoint.geometry, b,
                                                                  b.startSketchPoint.geometry, mm(5)).objectType)
out["fillet_after"] = curves(sk)
out["fillet_cons"] = cons(sk)
try_("fillet_too_big", lambda: sk.sketchCurves.sketchArcs.addFillet(a, a.startSketchPoint.geometry, b,
                                                                      b.endSketchPoint.geometry, mm(50)).objectType)

# 9. projected geometry: project body edge, then delete the body feature
sk0 = root.sketches.add(root.xYConstructionPlane)
sk0.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(mm(20), mm(10), 0))
ext = root.features.extrudeFeatures.addSimple(sk0.profiles.item(0), adsk.core.ValueInput.createByString("5 mm"),
                                              adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
sk = root.sketches.add(root.xZConstructionPlane)
sk.name = "projected"
body = ext.bodies.item(0)
proj = sk.project(body.edges.item(0))
out["projected_count"] = proj.count
out["projected_isReference"] = [proj.item(i).isReference for i in range(proj.count)]
out["projected_isLinked"] = [proj.item(i).isLinked for i in range(proj.count)]
sk.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(mm(5), mm(5), 0))
ext.deleteMe()
out["after_delete_curves"] = sk.sketchCurves.count
out["after_delete_health"] = [sk.healthState, sk.errorOrWarningMessage[:300]] if True else None
out["after_delete_curve_states"] = [(sk.sketchCurves.item(i).isReference, sk.sketchCurves.item(i).isLinked)
                                    for i in range(sk.sketchCurves.count)]
result.update(out)
result["doc"] = doc.name
