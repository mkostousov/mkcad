# MKProbe job: compact document state — active edit object, sketches (visible, profiles, isComputeDeferred),
# timeline items, undo-relevant counters, camera.
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sks = []
for i in range(root.sketches.count):
    s = root.sketches.item(i)
    sks.append({"n": s.name, "vis": s.isVisible, "prof": s.profiles.count, "curves": s.sketchCurves.count,
                "fully": s.isFullyConstrained, "health": s.healthState})
tl = design.timeline
result["active"] = app.activeEditObject.objectType.split("::")[-1]
result["sketches"] = sks
result["timeline"] = [tl.item(i).name for i in range(tl.count)]
result["marker"] = tl.markerPosition
result["bodies"] = root.bRepBodies.count
result["sel"] = ui.activeSelections.count
result["cmd"] = ui.activeCommand
cam = app.activeViewport.camera
result["eye"] = [round(cam.eye.x, 2), round(cam.eye.y, 2), round(cam.eye.z, 2)]
result["camType"] = cam.cameraType
result["doc"] = [app.activeDocument.name, app.activeDocument.isModified]
result["vol"] = round(root.bRepBodies.item(0).volume, 4) if root.bRepBodies.count else None
