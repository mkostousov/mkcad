# MKProbe job: select all curves of the active sketch, then start AutoConstrain.
sk = adsk.fusion.Sketch.cast(app.activeEditObject)
ui.activeSelections.clear()
for i in range(sk.sketchCurves.count):
    ui.activeSelections.add(sk.sketchCurves.item(i))
ui.commandDefinitions.itemById("SketchAutoConstraintAndDimCmd").execute()
result["sel"] = ui.activeSelections.count
result["cmd"] = ui.activeCommand
