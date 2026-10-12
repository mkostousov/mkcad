# MKProbe job: timeline items, bodies and selection count of the active design.
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
result["timeline"] = [design.timeline.item(i).name for i in range(design.timeline.count)]
result["bodies"] = [round(root.bRepBodies.item(i).volume * 1000, 2) for i in range(root.bRepBodies.count)]
result["selection"] = ui.activeSelections.count
result["activeCommand"] = ui.activeCommand
