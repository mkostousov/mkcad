# MKProbe job X-01: which piece does a split-face profile resolve to (F116/F117)?
# Same setup as face_split.py; additionally logs sketch origin/axes and the face's pointOnFace before the split,
# then the x/y range of the surviving cut piece, to test "piece containing sketch origin / pointOnFace".
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
OP = adsk.fusion.FeatureOperations

def mm(p):
    return [round(p.x * 10, 3), round(p.y * 10, 3), round(p.z * 10, 3)]

def run_case(tag, cx, gx, along_x=False):
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
    design = adsk.fusion.Design.cast(app.activeProduct)
    design.designType = adsk.fusion.DesignTypes.ParametricDesignType
    root = design.rootComponent
    ext = root.features.extrudeFeatures
    sk1 = root.sketches.add(root.xYConstructionPlane)
    sk1.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
    e1 = ext.addSimple(sk1.profiles.item(0), VI("10 mm"), OP.NewBodyFeatureOperation)
    body = e1.bodies.item(0)
    top = max([body.faces.item(i) for i in range(body.faces.count)], key=lambda f: f.pointOnFace.z)
    out = {"face_pointOnFace": mm(top.pointOnFace)}
    sk2 = root.sketches.add(top)
    out["sk_origin"] = mm(sk2.origin)
    out["sk_xdir"] = [round(v, 3) for v in sk2.xDirection.asArray()]
    out["sk_ydir"] = [round(v, 3) for v in sk2.yDirection.asArray()]
    c = sk2.modelToSketchSpace(P3(cx / 10, 1, 1))
    sk2.sketchCurves.sketchCircles.addByCenterRadius(c, 0.2)
    pr = sk2.profiles.item(0)
    out["profile0_loops"] = pr.profileLoops.count
    cut = ext.addSimple(pr, VI("-2 mm"), OP.CutFeatureOperation)
    tl = design.timeline
    tl.markerPosition = 2
    pl_in = root.constructionPlanes.createInput()
    pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
    pl = root.constructionPlanes.add(pl_in)
    sk3 = root.sketches.add(pl)
    if along_x:
        sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(-0.1, (gx - 2) / 10, 0), P3(4.1, (gx + 2) / 10, 0))
    else:
        sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3((gx - 2) / 10, -0.1, 0), P3((gx + 2) / 10, 2.1, 0))
    ext.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    b = None
    for i in range(cut.faces.count):
        fb = cut.faces.item(i).boundingBox
        b = fb if b is None else b
        b.combine(fb)
    out["cut_xy_mm"] = [round(b.minPoint.x * 10, 2), round(b.minPoint.y * 10, 2), round(b.maxPoint.x * 10, 2), round(b.maxPoint.y * 10, 2)]
    out["sk_origin_after"] = mm(sk2.origin)
    out["cut_health"] = cut.healthState
    result[tag] = out

for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)

run_case("gx30", 10, 30)
run_case("gx10", 10, 10)
run_case("gy6", 10, 6, True)
run_case("gy14", 10, 14, True)
run_case("gx10_c30", 30, 10)   # circle at x=30, groove at 8-12: does piece follow circle?

for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
