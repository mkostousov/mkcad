# MKProbe job (X-02): face reference of a Shell when its removed face is split upstream.
# Box 40x20x10 mm; Shell removing the top face, thickness 1 mm. Insert upstream (marker after Extrude1) a groove
# 4 mm wide x 3 mm deep across the top face at x = 28..32, move to end, report shell faces/health/volume.
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
OP = adsk.fusion.FeatureOperations

for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
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
oc = adsk.core.ObjectCollection.create()
oc.add(top)
si = root.features.shellFeatures.createInput(oc, False)
si.insideThickness = VI("1 mm")
sh = root.features.shellFeatures.add(si)

def vol():
    return round(sum(root.bRepBodies.item(i).volume for i in range(root.bRepBodies.count)) * 1000, 3)

def openings():
    # faces of the body lying at z = 10 mm (the rim) and at z = 7 mm (groove floor), as bounding boxes
    b = root.bRepBodies.item(0)
    out = []
    for i in range(b.faces.count):
        f = b.faces.item(i)
        bb = f.boundingBox
        if abs(bb.maxPoint.z - bb.minPoint.z) < 1e-6:
            out.append([round(v * 10, 2) for v in (bb.minPoint.x, bb.minPoint.y, bb.minPoint.z, bb.maxPoint.x, bb.maxPoint.y)] + [round(f.area * 100, 2)])
    return sorted(out)

result["before"] = {"vol": vol(), "health": sh.healthState, "flat_faces": openings()}
tl = design.timeline
tl.markerPosition = 2
pl_in = root.constructionPlanes.createInput()
pl_in.setByOffset(root.xYConstructionPlane, VI("10 mm"))
pl = root.constructionPlanes.add(pl_in)
sk3 = root.sketches.add(pl)
sk3.sketchCurves.sketchLines.addTwoPointRectangle(P3(2.8, -0.1, 0), P3(3.2, 2.1, 0))
g = ext.addSimple(sk3.profiles.item(0), VI("-3 mm"), OP.CutFeatureOperation)
result["groove_only_vol"] = vol()
tl.moveToEnd()
result["after"] = {"vol": vol(), "health": sh.healthState, "msg": sh.errorOrWarningMessage,
                   "bodies": root.bRepBodies.count, "flat_faces": openings()}

