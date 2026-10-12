# MKProbe job: in the active sketch, preselect the circle, then start the sketch Mirror command.
sk = adsk.fusion.Sketch.cast(app.activeEditObject)
ui.activeSelections.clear()
ui.activeSelections.add(sk.sketchCurves.sketchCircles.item(0))
ui.commandDefinitions.itemById("MirrorSketchCommand").execute()
result["sel"] = ui.activeSelections.count
result["active"] = app.activeEditObject.objectType
