# MKProbe job: face-reference behaviour when an upstream groove splits the face a sketch sits on (G-Q2).
# Box 40x20x10 mm; Sketch2 on top face with circle r=2 at (cx,10); Cut 2 mm. Then insert upstream (marker after
# Extrude1) a groove 4 mm wide x 3 mm deep across the top face at x = gx-2..gx+2, move to end, report.
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
OP = adsk.fusion.FeatureOperations

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
    sk2 = root.sketches.add(top)
    # sketch coords on a face may differ from model coords: convert a model point
    c = sk2.modelToSketchSpace(P3(cx / 10, 1, 1))
    sk2.sketchCurves.sketchCircles.addByCenterRadius(c, 0.2)
    cut = ext.addSimple(sk2.profiles.item(0), VI("-2 mm"), OP.CutFeatureOperation)
    def bb(f):
        b = f.boundingBox
        return [round(v * 10, 2) for v in (b.minPoint.x, b.minPoint.y, b.minPoint.z, b.maxPoint.x, b.maxPoint.y, b.maxPoint.z)]
    def circ():
        ci = sk2.sketchCurves.sketchCircles.item(0)
        w = sk2.sketchToModelSpace(ci.centerSketchPoint.geometry)
        return [round(w.x * 10, 3), round(w.y * 10, 3), round(w.z * 10, 3), round(ci.radius * 10, 3)]
    out = {"vol_box": 8000.0, "vol_before": round(root.bRepBodies.item(0).volume * 1000, 3),
           "circle_before": circ(), "cut_bb_before": [bb(cut.faces.item(i)) for i in range(cut.faces.count)]}
    tl = design.timeline
    tl.markerPosition = 2
    pl_in = root.constructionPlanes.createInput()
    pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
    pl = root.constructionPlanes.add(pl_in)
    sk3 = root.sketches.add(pl)
    if along_x:   # groove parallel to X at y = gx-2..gx+2
        sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(-0.1, (gx - 2) / 10, 0), P3(4.1, (gx + 2) / 10, 0))
    else:
        sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3((gx - 2) / 10, -0.1, 0), P3((gx + 2) / 10, 2.1, 0))
    g = ext.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
    tl.moveToEnd()
    out["timeline"] = [tl.item(i).name for i in range(tl.count)]
    out["groove_bb"] = [bb(g.faces.item(i)) for i in range(g.faces.count)]
    out["circle_after"] = circ()
    out["cut_bb_after"] = [bb(cut.faces.item(i)) for i in range(cut.faces.count)]
    out["sk2_health"] = sk2.healthState
    out["sk2_msg"] = sk2.errorOrWarningMessage
    out["sk2_plane_z_mm"] = round(sk2.origin.z * 10, 3)
    out["cut_health"] = cut.healthState
    out["cut_msg"] = cut.errorOrWarningMessage
    out["cut_faces"] = cut.faces.count
    out["bodies"] = root.bRepBodies.count
    out["vol_after"] = round(sum(root.bRepBodies.item(i).volume for i in range(root.bRepBodies.count)) * 1000, 3)
    result[tag] = out

for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)

run_case("groove_y6", 10, 6, True)    # pieces y 0..4 (small) and 8..20 (large)
run_case("groove_y14", 10, 14, True)  # pieces y 0..12 (large) and 16..20 (small)


