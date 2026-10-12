# MKProbe job (X-03): edge fillet r=1 on the top +y edge of a 40x20x10 box; an upstream 4x20x3 mm groove at x 28..32
# splits that edge (F42: both segments filleted). Then the groove is (a) suppressed, (b) unsuppressed, (c) deleted.
# Does the fillet return to one face over the full edge, and does it survive re-splitting? 3 fresh runs.
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

def fil_faces(root):
    r = []
    for b in range(root.bRepBodies.count):
        body = root.bRepBodies.item(b)
        for f in body.faces:
            if f.geometry.surfaceType != adsk.core.SurfaceTypes.PlaneSurfaceType:
                bb = f.boundingBox
                r.append("%g-%g" % (round(bb.minPoint.x * 10, 2), round(bb.maxPoint.x * 10, 2)))
    return sorted(r)

def state(root, f):
    return {"vol": vol(root), "fil": fil_faces(root), "health": f.healthState, "msg": f.errorOrWarningMessage,
            "edges": None}

def run_case(tag):
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
    design = adsk.fusion.Design.cast(app.activeProduct)
    design.designType = adsk.fusion.DesignTypes.ParametricDesignType
    root = design.rootComponent
    ext = root.features.extrudeFeatures
    sk1 = root.sketches.add(root.xYConstructionPlane)
    sk1.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
    body = ext.addSimple(sk1.profiles.item(0), VI("10 mm"), OP.NewBodyFeatureOperation).bodies.item(0)
    fs = list(body.faces)
    top = max(fs, key=lambda f: f.pointOnFace.z)
    side = max(fs, key=lambda f: f.pointOnFace.y)
    edge = [e for e in top.edges if any(x == side for x in e.faces)][0]
    coll = adsk.core.ObjectCollection.create()
    coll.add(edge)
    fil = root.features.filletFeatures
    fi = fil.createInput()
    fi.edgeSetInputs.addConstantRadiusEdgeSet(coll, VI("1 mm"), False)
    f = fil.add(fi)
    out = {}
    result[tag] = out
    out["0_initial"] = state(root, f)
    tl = design.timeline
    tl.markerPosition = 2
    pl_in = root.constructionPlanes.createInput()
    pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
    pl = root.constructionPlanes.add(pl_in)
    sk3 = root.sketches.add(pl)
    sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(2.8, -0.1, 0), P3(3.2, 2.1, 0))
    g = ext.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    out["1_split"] = state(root, f)
    g.timelineObject.isSuppressed = True
    out["2_suppressed"] = state(root, f)
    g.timelineObject.isSuppressed = False
    out["3_unsuppressed"] = state(root, f)
    g.deleteMe()
    out["4_deleted"] = state(root, f)
    # re-split with a new groove after deletion
    tl.markerPosition = 3
    sk4 = root.sketches.add(pl)
    sk4.sketchCurves.sketchLines.addTwoPointRectangle(P3(0.8, -0.1, 0), P3(1.2, 2.1, 0))
    ext.addSimple(sk4.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    out["5_new_split"] = state(root, f)
    out["timeline"] = [tl.item(i).name for i in range(tl.count)]
    result[tag] = out

close_unsaved()
for k in range(3):
    try:
        run_case("run%d" % (k + 1))
    except Exception as ex:
        result.setdefault("run%d" % (k + 1), {})["error"] = str(ex)
    close_unsaved()
