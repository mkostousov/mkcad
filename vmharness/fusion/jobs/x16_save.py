# MKProbe job: X-16 — build a part with known state, save it (uses 1 editable-document slot), report state.
import json
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
design.userParameters.add("Len", VI("40 mm"), "mm", "persist test")
sk = root.sketches.add(root.xYConstructionPlane)
ls = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
d = sk.sketchDimensions.addDistanceDimension(ls.item(0).startSketchPoint, ls.item(0).endSketchPoint,
                                             adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0))
d.parameter.expression = "Len"
ext = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
body = ext.bodies.item(0)
top = max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.centroid.z)
c = adsk.core.ObjectCollection.create()
for i in range(top.edges.count):
    c.add(top.edges.item(i))
fi = root.features.filletFeatures.createInput(); fi.addConstantRadiusEdgeSet(c, VI("2 mm"), True)
root.features.filletFeatures.add(fi)
design.timeline.item(1).rollTo(False)          # marker after Extrude1 (fillet rolled back)
vp = app.activeViewport
cam = vp.camera
cam.isSmoothTransition = False
cam.eye = P3(30, -40, 25); cam.target = P3(2, 1, 0.5); cam.upVector = adsk.core.Vector3D.create(0, 0, 1)
vp.camera = cam
nv = design.namedViews.add(vp.camera, "MK_View")
ui.activeSelections.clear(); ui.activeSelections.add(body)
projs = [app.data.dataProjects.item(i) for i in range(app.data.dataProjects.count)]
result["projects"] = [p.name for p in projs]
proj = next(p for p in projs if p.name == "Default Project")
folder = proj.rootFolder
ok = doc.saveAs("mk_x16_persist", folder, "MK clean-room study X-16", "")
adsk.doEvents()
result["saveAs"] = ok
result["project"] = proj.name
result["doc"] = {"name": doc.name, "isSaved": doc.isSaved, "isModified": doc.isModified,
                 "version": doc.dataFile.versionNumber if doc.dataFile else None,
                 "id": doc.dataFile.id if doc.dataFile else None}
cam = vp.camera
result["before"] = {"marker": design.timeline.markerPosition, "timeline": design.timeline.count,
                    "eye": [round(cam.eye.x, 3), round(cam.eye.y, 3), round(cam.eye.z, 3)],
                    "namedViews": [design.namedViews.item(i).name for i in range(design.namedViews.count)],
                    "selection": ui.activeSelections.count, "params": [p.name for p in design.userParameters]}

