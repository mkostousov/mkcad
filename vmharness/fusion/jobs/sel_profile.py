# MKProbe job: move marker to end, preselect the first profile of the last sketch (for a GUI Extrude).
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
design.timeline.moveToEnd()
sk = root.sketches.item(root.sketches.count - 1)
ui.activeSelections.clear()
ui.activeSelections.add(sk.profiles.item(0))
result["sketch"] = sk.name
result["sel"] = ui.activeSelections.count
result["marker"] = design.timeline.markerPosition
