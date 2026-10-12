# MKProbe job (X-02): face reference of an Offset Face (+2 mm on the top face of a 40x20x10 box) when that face
# is split upstream by a 4x20x3 mm groove at x 28..32. Volume tells which pieces are offset:
# none 7760, both 9200, x 0..28 only 8880, x 32..40 only 8080 (mm^3). Run 3 times in fresh documents.
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
OP = adsk.fusion.FeatureOperations

def close_unsaved():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)

def vol(root):
    return round(sum(root.bRepBodies.item(i).volume for i in range(root.bRepBodies.count)) * 1000, 3)

def run_case(tag):
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
    off = root.features.offsetFacesFeatures
    of = off.add(off.createInput([top], VI("2 mm")))
    out = {"vol_before": vol(root)}
    tl = design.timeline
    tl.markerPosition = 2
    pl_in = root.constructionPlanes.createInput()
    pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
    pl = root.constructionPlanes.add(pl_in)
    sk3 = root.sketches.add(pl)
    sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(2.8, -0.1, 0), P3(3.2, 2.1, 0))
    ext.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    out["vol_after"] = vol(root)
    out["health"] = of.healthState
    out["msg"] = of.errorOrWarningMessage
    out["bodies"] = root.bRepBodies.count
    out["timeline"] = [tl.item(i).name for i in range(tl.count)]
    result[tag] = out

close_unsaved()
for k in range(3):
    run_case("run%d" % (k + 1))
    close_unsaved()
