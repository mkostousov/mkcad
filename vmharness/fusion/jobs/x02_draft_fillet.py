# MKProbe job (X-02 remainder): face references of Draft and of a face-set Fillet when the referenced face is split
# upstream by a 4x20x3 mm groove at x 28..32 (40x20x10 box, groove inserted via rollback). 3 fresh runs each.
# Draft: top face (z=10), 5 deg, pull plane = XZ -> tilted +z faces (x ranges) tell which pieces are drafted.
# Fillet: constant r=1 with the TOP FACE as input entity (fillets all its edges) -> fillet faces by x range.
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

def faces(root):
    out = []
    for b in range(root.bRepBodies.count):
        body = root.bRepBodies.item(b)
        for i in range(body.faces.count):
            out.append(body.faces.item(i))
    return out

def base(tag):
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
    design = adsk.fusion.Design.cast(app.activeProduct)
    design.designType = adsk.fusion.DesignTypes.ParametricDesignType
    root = design.rootComponent
    sk1 = root.sketches.add(root.xYConstructionPlane)
    sk1.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
    e1 = root.features.extrudeFeatures.addSimple(sk1.profiles.item(0), VI("10 mm"), OP.NewBodyFeatureOperation)
    return design, root, e1.bodies.item(0)

def groove(design, root):
    tl = design.timeline
    tl.markerPosition = 2
    pl_in = root.constructionPlanes.createInput()
    pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
    pl = root.constructionPlanes.add(pl_in)
    sk3 = root.sketches.add(pl)
    sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(2.8, -0.1, 0), P3(3.2, 2.1, 0))
    root.features.extrudeFeatures.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    return [tl.item(i).name for i in range(tl.count)]

def tilted_y_faces(root):
    # planar faces whose normal is mostly +y but not exactly (drafted) -> list of x ranges
    r = []
    for f in faces(root):
        ok, n = f.evaluator.getNormalAtPoint(f.pointOnFace)
        if ok and n.z > 0.5 and abs(n.y) > 1e-4:
            bb = f.boundingBox
            r.append([round(bb.minPoint.x * 10, 2), round(bb.maxPoint.x * 10, 2), round(n.y, 4)])
    return sorted(r)

def fillet_faces(root):
    r = []
    for f in faces(root):
        if f.geometry.surfaceType != adsk.core.SurfaceTypes.PlaneSurfaceType:
            bb = f.boundingBox
            r.append([round(bb.minPoint.x * 10, 2), round(bb.maxPoint.x * 10, 2), round(bb.minPoint.y * 10, 2), round(bb.maxPoint.y * 10, 2), f.geometry.surfaceType])
    return sorted(r)

def run_draft(tag):
    design, root, body = base(tag)
    top = max([body.faces.item(i) for i in range(body.faces.count)], key=lambda f: f.pointOnFace.z)
    dfs = root.features.draftFeatures
    di = dfs.createInput([top], root.xZConstructionPlane, True)
    di.setSingleAngle(False, VI("5 deg"))
    d = dfs.add(di)
    out = {"vol_before": vol(root), "tilted_before": tilted_y_faces(root)}
    out["timeline"] = groove(design, root)
    out["vol_after"] = vol(root)
    out["tilted_after"] = tilted_y_faces(root)
    out["health"] = d.healthState
    out["msg"] = d.errorOrWarningMessage
    out["bodies"] = root.bRepBodies.count
    result[tag] = out

def run_fillet(tag):
    design, root, body = base(tag)
    top = max(list(body.faces), key=lambda f: f.pointOnFace.z)
    fil = root.features.filletFeatures
    fi = fil.createInput()
    coll = adsk.core.ObjectCollection.create()
    coll.add(top)
    fi.edgeSetInputs.addConstantRadiusEdgeSet(coll, VI("1 mm"), False)
    f = fil.add(fi)
    out = {"vol_before": vol(root), "fil_before": fillet_faces(root)}
    out["timeline"] = groove(design, root)
    out["vol_after"] = vol(root)
    out["fil_after"] = fillet_faces(root)
    out["health"] = f.healthState
    out["msg"] = f.errorOrWarningMessage
    out["bodies"] = root.bRepBodies.count
    result[tag] = out

close_unsaved()
for k in range(3):
    for name, fn in (("draft", run_draft), ("fillet", run_fillet)):
        try:
            fn("%s_run%d" % (name, k + 1))
        except Exception as ex:
            result["%s_run%d" % (name, k + 1)] = {"error": str(ex)}
        close_unsaved()
