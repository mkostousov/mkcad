# MKProbe job: persistent-reference behaviour of a fillet when upstream topology changes (spec §5 topological naming).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput
H = adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation
Vt = adsk.fusion.DimensionOrientations.VerticalDimensionOrientation
NEW = adsk.fusion.FeatureOperations.NewBodyFeatureOperation
CUT = adsk.fusion.FeatureOperations.CutFeatureOperation
steps = []

app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
tl = design.timeline


def r(v):
    return round(v * 10, 3)  # cm -> mm


def rect_sketch(plane, x0, y0, x1, y1):
    sk = root.sketches.add(plane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(x0, y0, 0), P3(x1, y1, 0))
    return sk


def fillet_info(f):
    info = {"health": {0: "Healthy", 1: "Warning", 2: "Error", 3: "Suppressed"}.get(f.healthState, f.healthState)}
    try:
        if f.errorOrWarningMessage:
            info["msg"] = f.errorOrWarningMessage[:300]
    except Exception:
        pass
    try:
        faces = f.faces
        info["faces"] = faces.count
        info["face_centroids_mm"] = [[r(faces.item(i).centroid.x), r(faces.item(i).centroid.y), r(faces.item(i).centroid.z)]
                                     for i in range(faces.count)]
    except Exception as e:
        info["faces"] = f"<{e}>"
    return info


def state(label, fillet, **extra):
    b = root.bRepBodies
    s = {"step": label, "fillet": fillet_info(fillet),
         "bodies": [{"vol_mm3": round(b.item(i).volume * 1000, 2), "faces": b.item(i).faces.count} for i in range(b.count)],
         "timeline": [tl.item(i).name for i in range(tl.count)]}
    s.update(extra)
    steps.append(s)


# Base box 40 x 20 x 10 mm (cm units in API)
sk = rect_sketch(root.xYConstructionPlane, 0, 0, 4, 2)
ext = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI.createByString("10 mm"), NEW)
body = ext.bodies.item(0)
# pick the vertical edge at x=40, y=20
edge = None
for i in range(body.edges.count):
    e = body.edges.item(i)
    sp, ep = e.startVertex.geometry, e.endVertex.geometry
    if abs(sp.x - 4) < 1e-6 and abs(ep.x - 4) < 1e-6 and abs(sp.y - 2) < 1e-6 and abs(ep.y - 2) < 1e-6:
        edge = e
coll = adsk.core.ObjectCollection.create(); coll.add(edge)
fi = root.features.filletFeatures.createInput()
fi.addConstantRadiusEdgeSet(coll, VI.createByString("3 mm"), False)
fillet = root.features.filletFeatures.add(fi)
state("fillet r3 on vertical edge at (40,20)", fillet)

# 1) unrelated hole inserted BEFORE the fillet
tl.item(ext.timelineObject.index).rollTo(False)
skh = root.sketches.add(root.xYConstructionPlane)
skh.sketchCurves.sketchCircles.addByCenterRadius(P3(1, 1, 0), 0.4)
hole = root.features.extrudeFeatures.addSimple(skh.profiles.item(0), VI.createByString("10 mm"), CUT)
tl.moveToEnd()
state("unrelated through-hole at (10,10) inserted before fillet", fillet)

# 2) groove that splits the filleted edge into two segments (cut on XZ plane through full width at z 4..6 mm)
tl.item(hole.timelineObject.index).rollTo(False)
xz = root.xZConstructionPlane
skg = root.sketches.add(xz)
# XZ sketch: sketch x = model x, sketch y = model -z (Fusion XZ plane orientation); use modelToSketchSpace to be safe
p0 = skg.modelToSketchSpace(P3(-0.5, 0, 0.4)); p1 = skg.modelToSketchSpace(P3(4.5, 0, 0.6))
skg.sketchCurves.sketchLines.addTwoPointRectangle(p0, p1)
ext_in = root.features.extrudeFeatures.createInput(skg.profiles.item(0), CUT)
ext_in.setDistanceExtent(True, VI.createByString("30 mm"))  # symmetric, through the 20 mm depth
groove = root.features.extrudeFeatures.add(ext_in)
tl.moveToEnd()
state("groove z=4..6 mm across the part splits the filleted edge", fillet)

# 3) remove the filleted edge entirely: corner cut 10x10 mm at (40,20) inserted before fillet
tl.item(groove.timelineObject.index).rollTo(False)
skc = rect_sketch(root.xYConstructionPlane, 3, 1, 4.5, 2.5)
corner = root.features.extrudeFeatures.addSimple(skc.profiles.item(0), VI.createByString("10 mm"), CUT)
tl.moveToEnd()
state("corner notch removes the filleted edge", fillet)

# 4) delete the notch again -> does the fillet recover?
corner.deleteMe()
state("notch deleted -> fillet recovery?", fillet)
result["steps"] = steps
