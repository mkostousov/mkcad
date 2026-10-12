# MKProbe job: signalling test — STAGE "under": fresh doc, rectangle with H/V only + a free line, sketch edit, top view.
# STAGE "full": add dims + anchor so the rectangle is fully constrained (free line stays free); report flags.
STAGE = "__STAGE__"
P3 = adsk.core.Point3D.create
H = adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation
Vt = adsk.fusion.DimensionOrientations.VerticalDimensionOrientation
if STAGE == "under":
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
if STAGE == "under":
    sk = root.sketches.add(root.xYConstructionPlane)
    L = sk.sketchCurves.sketchLines
    a = L.addByTwoPoints(P3(1, 1, 0), P3(5, 1, 0)); b = L.addByTwoPoints(a.endSketchPoint, P3(5, 3, 0))
    c = L.addByTwoPoints(b.endSketchPoint, P3(1, 3, 0)); d = L.addByTwoPoints(c.endSketchPoint, a.startSketchPoint)
    g = sk.geometricConstraints
    g.addHorizontal(a); g.addHorizontal(c); g.addVertical(b); g.addVertical(d)
    L.addByTwoPoints(P3(6, 1, 0), P3(8, 3, 0))     # free line, stays under-constrained
    ui.activeSelections.clear(); ui.activeSelections.add(sk)
    ui.commandDefinitions.itemById("SketchActivate").execute()
    vp = app.activeViewport; cam = vp.camera
    cam.cameraType = adsk.core.CameraTypes.OrthographicCameraType
    cam.target = P3(4.5, 2, 0); cam.eye = P3(4.5, 2, 50); cam.upVector = adsk.core.Vector3D.create(0, 1, 0)
    cam.viewExtents = 6.0; cam.isSmoothTransition = False
    vp.camera = cam; vp.refresh(); adsk.doEvents()
else:
    sk = root.sketches.item(0)
    a, b = sk.sketchCurves.sketchLines.item(0), sk.sketchCurves.sketchLines.item(1)
    sk.geometricConstraints.addCoincident(a.startSketchPoint, sk.originPoint) if False else None
    a.startSketchPoint.isFixed = True
    sk.sketchDimensions.addDistanceDimension(a.startSketchPoint, a.endSketchPoint, H, P3(3, 0.3, 0))
    sk.sketchDimensions.addDistanceDimension(b.startSketchPoint, b.endSketchPoint, Vt, P3(5.8, 2, 0))
    ui.activeSelections.clear()
result["stage"] = STAGE
result["sketchFully"] = sk.isFullyConstrained
result["curvesFully"] = [sk.sketchCurves.item(i).isFullyConstrained for i in range(sk.sketchCurves.count)]
result["active"] = app.activeEditObject.objectType
