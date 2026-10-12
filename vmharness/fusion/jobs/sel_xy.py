# MKProbe job: add the XY origin plane to the active selection (fills the active command's selection input).
root = adsk.fusion.Design.cast(app.activeProduct).rootComponent
ui.activeSelections.add(root.xYConstructionPlane)
result["sel"] = ui.activeSelections.count
result["cmd"] = ui.activeCommand
