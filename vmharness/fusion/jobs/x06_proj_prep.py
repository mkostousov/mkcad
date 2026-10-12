# MKProbe job: X-06 projected geometry — box x0..40 y30..50 z0..10 mm, sketch on XY with the box's y=30 bottom edge
# projected, body hidden, sketch edit active, top ortho view like prep_infer (screen = (393+14.2x, 848-14.2y)).
P3 = adsk.core.Point3D.create
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
s0 = root.sketches.add(root.xYConstructionPlane)
s0.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 3, 0), P3(4, 5, 0))
body = root.features.extrudeFeatures.addSimple(s0.profiles.item(0), adsk.core.ValueInput.createByString("10 mm"),
                                               adsk.fusion.FeatureOperations.NewBodyFeatureOperation).bodies.item(0)
s0.isVisible = False
edge = None
for i in range(body.edges.count):
    e = body.edges.item(i)
    a, b = e.startVertex.geometry, e.endVertex.geometry
    if abs(a.y - 3) < 1e-6 and abs(b.y - 3) < 1e-6 and abs(a.z) < 1e-6 and abs(b.z) < 1e-6:
        edge = e
sk = root.sketches.add(root.xYConstructionPlane)
proj = sk.project(edge)
body.isLightBulbOn = False
result["projected"] = proj.count
ui.activeSelections.clear()
ui.activeSelections.add(sk)
ui.commandDefinitions.itemById("SketchActivate").execute()
vp = app.activeViewport
cam = vp.camera
cam.cameraType = adsk.core.CameraTypes.OrthographicCameraType
cam.target = P3(4, 2, 0)
cam.eye = P3(4, 2, 50)
cam.upVector = adsk.core.Vector3D.create(0, 1, 0)
cam.viewExtents = 6.0
cam.isSmoothTransition = False
vp.camera = cam
vp.refresh()
adsk.doEvents()
p = vp.modelToViewSpace(P3(4, 3, 0))
result["projEnd_screen"] = [round(p.x), round(p.y) + 140]
p = vp.modelToViewSpace(P3(2, 3, 0))
result["projMid_screen"] = [round(p.x), round(p.y) + 140]
result["activeEdit"] = app.activeEditObject.objectType
