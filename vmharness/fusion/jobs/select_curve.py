# MKProbe job: select curve #1 (the circle) of the last sketch, report counts before a GUI Delete.
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.item(root.sketches.count - 1)
ui.activeSelections.clear()
ui.activeSelections.add(sk.sketchCurves.sketchCircles.item(0))
result["selected"] = ui.activeSelections.count
result["curves"] = sk.sketchCurves.count
result["points"] = sk.sketchPoints.count
result["cons"] = sk.geometricConstraints.count
