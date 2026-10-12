# MKProbe job: close unsaved docs, new doc with a 40x20 rectangle sketch, pre-select its profile for a GUI command.
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
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
ui.activeSelections.clear()
ui.activeSelections.add(sk.profiles.item(0))
app.activeViewport.fit()
result["selected"] = ui.activeSelections.count
result["docs"] = app.documents.count
