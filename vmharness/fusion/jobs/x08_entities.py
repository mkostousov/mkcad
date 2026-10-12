# MKProbe job (X-08): what each API sketch-entity creator adds to a sketch: curves (by type), points, geometric
# constraints, dimensions, profiles. One fresh sketch per entity kind; methods that do not exist are reported.
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByReal

def close_unsaved():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)

def snapshot(sk):
    curves = {}
    for i in range(sk.sketchCurves.count):
        c = sk.sketchCurves.item(i)
        k = c.objectType.split("::")[-1] + ("(constr)" if c.isConstruction else "")
        curves[k] = curves.get(k, 0) + 1
    cons = {}
    for i in range(sk.geometricConstraints.count):
        k = sk.geometricConstraints.item(i).objectType.split("::")[-1]
        cons[k] = cons.get(k, 0) + 1
    dims = [sk.sketchDimensions.item(i).objectType.split("::")[-1] + ":" + sk.sketchDimensions.item(i).parameter.expression
            for i in range(sk.sketchDimensions.count)]
    return {"curves": curves, "points": sk.sketchPoints.count, "constraints": cons, "dims": dims,
            "profiles": sk.profiles.count, "fully": sk.isFullyConstrained}

def creators(sk):
    L = sk.sketchCurves.sketchLines
    return {
        "ellipse": lambda: sk.sketchCurves.sketchEllipses.add(P3(0, 0, 0), P3(3, 0, 0), P3(1, 1, 0)),
        "fitted_spline": lambda: sk.sketchCurves.sketchFittedSplines.add(
            _coll([P3(0, 0, 0), P3(1, 1, 0), P3(2, 0, 0), P3(3, 1, 0)])),
        "control_spline": lambda: sk.sketchCurves.sketchControlPointSplines.add(
            [P3(0, 0, 0), P3(1, 1, 0), P3(2, 0, 0), P3(3, 1, 0)], adsk.fusion.SplineDegrees.SplineDegreeThree),
        "conic": lambda: sk.sketchCurves.sketchConicCurves.add(P3(0, 0, 0), P3(4, 0, 0), P3(2, 2, 0), 0.5),
        "slot_c2c": lambda: _slot(sk.addCenterToCenterSlot, P3(0, 0, 0), P3(3, 0, 0), adsk.core.ValueInput.createByReal(1.0)),
        "slot_overall": lambda: _slot(sk.addOverallSlot, P3(0, 0, 0), P3(3, 0, 0), adsk.core.ValueInput.createByReal(1.0)),
        "slot_center_point": lambda: _slot(sk.addCenterPointSlot, P3(0, 0, 0), P3(3, 0, 0), adsk.core.ValueInput.createByReal(1.0)),
        "slot_3pt_arc": lambda: _slot(sk.addThreePointArcSlot, P3(0, 0, 0), P3(1, 1, 0), P3(2, 0, 0), adsk.core.ValueInput.createByReal(0.5)),
        "slot_center_arc": lambda: _slot(sk.addCenterPointArcSlot, P3(0, 0, 0), P3(2, 0, 0), P3(0, 2, 0), adsk.core.ValueInput.createByReal(0.5)),
        "line_midpoint": lambda: L.addByMidpoint(P3(0, 0, 0), P3(2, 1, 0)),
        "polygon_scribed": lambda: L.addScribedPolygon(P3(0, 0, 0), 6, 0, 2.0, True),
        "polygon_edge": lambda: L.addEdgePolygon(P3(0, 0, 0), P3(2, 0, 0), True, 6),
        "rect_center": lambda: L.addCenterPointRectangle(P3(0, 0, 0), P3(2, 1, 0)),
        "rect_3pt": lambda: L.addThreePointRectangle(P3(0, 0, 0), P3(2, 1, 0), P3(1, 3, 0)),
        "circle_3pt": lambda: sk.sketchCurves.sketchCircles.addByThreePoints(P3(0, 0, 0), P3(2, 0, 0), P3(1, 1.5, 0)),
        "arc_3pt": lambda: sk.sketchCurves.sketchArcs.addByThreePoints(P3(0, 0, 0), P3(1, 1, 0), P3(2, 0, 0)),
        "arc_center": lambda: sk.sketchCurves.sketchArcs.addByCenterStartSweep(P3(0, 0, 0), P3(2, 0, 0), 1.5708),
        "text": lambda: sk.sketchTexts.add(sk.sketchTexts.createInput2("Ab", 1.0)) if False else None,
    }

def _slot(fn, *args):
    try:
        return fn(*args)
    except TypeError:
        return fn(*args, True)

def _coll(pts):
    c = adsk.core.ObjectCollection.create()
    for p in pts:
        c.add(p)
    return c

close_unsaved()
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
names = list(creators(root.sketches.add(root.xYConstructionPlane)).keys())
for name in names:
    if name == "text":
        continue
    sk = root.sketches.add(root.xYConstructionPlane)
    try:
        ret = creators(sk)[name]()
        r = snapshot(sk)
        r["returned"] = ("%s[%d]" % (type(ret).__name__, len(ret))) if not hasattr(ret, "objectType") and ret is not None else (ret.objectType.split("::")[-1] + ("[%d]" % ret.count if hasattr(ret, "count") else "")) if ret is not None else None
        result[name] = r
    except Exception as ex:
        result[name] = {"error": str(ex)[:300]}
close_unsaved()
