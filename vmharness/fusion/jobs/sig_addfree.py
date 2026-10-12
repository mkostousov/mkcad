# MKProbe job: add a free line to Sketch1 so the sketch becomes under-constrained again.
P3 = adsk.core.Point3D.create
sk = adsk.fusion.Design.cast(app.activeProduct).rootComponent.sketches.item(0)
sk.sketchCurves.sketchLines.addByTwoPoints(P3(6, 1, 0), P3(8, 3, 0))
result["sketchFully"] = sk.isFullyConstrained
