# MKProbe job: sketch constraint-solver behaviour probe (spec §4, §3 recompute).
# Runs in a fresh unsaved document. API lengths are in cm.
P3 = adsk.core.Point3D.create
V = adsk.core.ValueInput.createByReal
steps = []


def r(x, n=6):
    return round(x, n)


def snap(sk, label, extra=None):
    curves = []
    for i in range(sk.sketchCurves.count):
        c = sk.sketchCurves.item(i)
        e = {"type": c.objectType.split("::")[-1], "fully": c.isFullyConstrained, "construction": c.isConstruction}
        try:
            e["start"] = [r(c.startSketchPoint.geometry.x), r(c.startSketchPoint.geometry.y)]
            e["end"] = [r(c.endSketchPoint.geometry.x), r(c.endSketchPoint.geometry.y)]
        except Exception:
            pass
        curves.append(e)
    cons = {}
    for i in range(sk.geometricConstraints.count):
        t = sk.geometricConstraints.item(i).objectType.split("::")[-1]
        cons[t] = cons.get(t, 0) + 1
    dims = []
    for i in range(sk.sketchDimensions.count):
        d = sk.sketchDimensions.item(i)
        dims.append({"type": d.objectType.split("::")[-1], "driving": d.isDriving,
                     "expr": d.parameter.expression if d.parameter else None,
                     "value": r(d.parameter.value) if d.parameter else None})
    s = {"step": label, "fully": sk.isFullyConstrained, "profiles": sk.profiles.count,
         "constraints": cons, "dims": dims, "curves": curves}
    try:
        s["healthState"] = str(sk.healthState)
        s["errorOrWarningMessage"] = sk.errorOrWarningMessage
    except Exception as e:
        s["healthState"] = f"<{e}>"
    if extra:
        s.update(extra)
    steps.append(s)


def attempt(label, fn):
    try:
        out = fn()
        return {"attempt": label, "ok": True, "returned": None if out is None else out.objectType.split("::")[-1]}
    except Exception as e:
        return {"attempt": label, "ok": False, "error": str(e).strip()[:500]}


doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
snap(sk, "empty sketch on XY")

lines = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
snap(sk, "two-point rectangle 40x20mm at origin corner")

# Anchor corner to origin explicitly
attempt_res = attempt("coincident corner->origin",
                      lambda: sk.geometricConstraints.addCoincident(lines.item(0).startSketchPoint, sk.originPoint))
snap(sk, "corner coincident to origin", attempt_res)

dims = sk.sketchDimensions
top, right = lines.item(0), lines.item(1)
w = attempt("width dim", lambda: dims.addDistanceDimension(top.startSketchPoint, top.endSketchPoint,
            adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0)))
snap(sk, "width dimension", w)
h = attempt("height dim", lambda: dims.addDistanceDimension(right.startSketchPoint, right.endSketchPoint,
            adsk.fusion.DimensionOrientations.VerticalDimensionOrientation, P3(5, 1, 0)))
snap(sk, "height dimension", h)

# Redundant: second width dimension on the opposite edge (same value)
bottom = lines.item(2)
red = attempt("redundant dim (opposite edge)", lambda: dims.addDistanceDimension(
    bottom.startSketchPoint, bottom.endSketchPoint,
    adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, 3, 0)))
snap(sk, "after redundant dimension attempt", red)

# Conflicting geometric constraint: make a horizontal edge vertical
conf = attempt("conflicting vertical on horizontal edge", lambda: sk.geometricConstraints.addVertical(top))
snap(sk, "after conflicting constraint attempt", conf)

# Determinism: change width back and forth
wdim = sk.sketchDimensions.item(0)
coords = set()
for i in range(20):
    wdim.parameter.value = 6.0 if i % 2 == 0 else 4.0
    c = tuple((r(l.startSketchPoint.geometry.x, 9), r(l.startSketchPoint.geometry.y, 9))
              for l in [lines.item(k) for k in range(4)])
    if i % 2 == 1:
        coords.add(c)
snap(sk, "after 20 alternating width edits (end 40mm)", {"distinct_results_at_40mm": len(coords)})

# Extrude and recompute downstream
prof = sk.profiles.item(0)
ext = root.features.extrudeFeatures.addSimple(prof, adsk.core.ValueInput.createByString("10 mm"),
                                              adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
body = ext.bodies.item(0)
vol0 = body.volume
wdim.parameter.expression = "55 mm"
vol1 = root.bRepBodies.item(0).volume
snap(sk, "extruded 10mm then width->55mm", {"volume_cm3_before": r(vol0), "volume_cm3_after": r(vol1),
                                           "timeline_count": design.timeline.count})

# Open contour: sketch with a single line -> profiles
sk2 = root.sketches.add(root.xZConstructionPlane)
sk2.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(3, 0, 0))
snap(sk2, "second sketch: single open line on XZ")

result["steps"] = steps
result["document"] = {"name": doc.name, "isSaved": doc.isSaved}
