# MKProbe job: delete the free (last) line of Sketch1 so the whole sketch becomes fully constrained.
sk = adsk.fusion.Design.cast(app.activeProduct).rootComponent.sketches.item(0)
ls = sk.sketchCurves.sketchLines
ls.item(ls.count - 1).deleteMe()
result["sketchFully"] = sk.isFullyConstrained
