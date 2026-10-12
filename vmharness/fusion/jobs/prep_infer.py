# MKProbe job: fresh doc, sketch on XY with a reference line (0,0)-(40,0) mm and a circle c(60,30) r10,
# enter sketch edit, top view, report model->screen mapping (viewport px; add canvas offset on screen).
P3 = adsk.core.Point3D.create
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(4, 0, 0))
sk.sketchCurves.sketchCircles.addByCenterRadius(P3(6, 3, 0), 1)
ui.activeSelections.clear()
ui.activeSelections.add(sk)
try:
    ui.commandDefinitions.itemById("SketchActivate").execute()
    result["activate"] = "executed"
except Exception as ex:
    result["activate"] = str(ex)[:150]
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
m = {}
for name, x, y in (("o", 0, 0), ("x10", 1, 0), ("y10", 0, 1), ("lineEnd", 4, 0), ("cc", 6, 3)):
    p = vp.modelToViewSpace(P3(x, y, 0))
    m[name] = [round(p.x, 1), round(p.y, 1)]
result["map"] = m
result["vp"] = [vp.width, vp.height]
result["activeEdit"] = app.activeEditObject.objectType
