# MKProbe job: marker to end, add Sketch3 on XY with a circle far from the body (centre 100,100 mm), preselect it.
P3 = adsk.core.Point3D.create
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
design.timeline.moveToEnd()
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchCircles.addByCenterRadius(P3(10, 10, 0), 0.5)
ui.activeSelections.clear()
ui.activeSelections.add(sk.profiles.item(0))
app.activeViewport.fit()
result["sketch"] = sk.name
result["sel"] = ui.activeSelections.count
