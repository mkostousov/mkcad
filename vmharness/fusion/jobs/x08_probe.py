app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
root = adsk.fusion.Design.cast(app.activeProduct).rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
result["sketch"] = [m for m in dir(sk) if not m.startswith("_") and ("lot" in m or "ext" in m or "oly" in m)]
result["curves"] = [m for m in dir(sk.sketchCurves) if not m.startswith("_")]
result["lines"] = [m for m in dir(sk.sketchCurves.sketchLines) if m.startswith("add")]
result["gc"] = [m for m in dir(sk.geometricConstraints) if m.startswith("add")]
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved: d.close(False)
