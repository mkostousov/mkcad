# MKProbe job: close unsaved docs, new doc: Sketch1 40x20 rect -> Extrude1 10 mm, Sketch2 circle on XY -> Extrude2 cut.
# Leaves the timeline marker at the end; used for marker/undo experiments (E-Q2, E-Q3).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
ext = root.features.extrudeFeatures
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
e1 = ext.addSimple(sk.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
sk2 = root.sketches.add(root.xYConstructionPlane)
sk2.sketchCurves.sketchCircles.addByCenterRadius(P3(2, 1, 0), 0.5)
e2 = ext.addSimple(sk2.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.CutFeatureOperation)
app.activeViewport.fit()
tl = design.timeline
result["timeline"] = [tl.item(i).name for i in range(tl.count)]
result["marker"] = tl.markerPosition
result["vol"] = round(root.bRepBodies.item(0).volume, 4)
