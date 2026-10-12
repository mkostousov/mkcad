# MKProbe job: move the timeline marker to position 2 (after Extrude1) via API, report state.
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
tl = design.timeline
tl.markerPosition = 2
result["marker"] = tl.markerPosition
result["rolled"] = [tl.item(i).isRolledBack for i in range(tl.count)]
result["vol"] = round(root.bRepBodies.item(0).volume, 4) if root.bRepBodies.count else None
