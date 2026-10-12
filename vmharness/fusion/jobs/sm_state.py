# MKProbe job: sheet-metal state of the active design — bodies, thickness, rule, flat pattern, timeline.
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
bodies = []
for i in range(root.bRepBodies.count):
    b = root.bRepBodies.item(i)
    bb = b.boundingBox
    bodies.append({"name": b.name, "vol_mm3": round(b.volume * 1000, 3), "faces": b.faces.count,
                   "isSheetMetal": getattr(b, "isSheetMetal", None),
                   "bbox_mm": [round((bb.maxPoint.x - bb.minPoint.x) * 10, 3), round((bb.maxPoint.y - bb.minPoint.y) * 10, 3),
                               round((bb.maxPoint.z - bb.minPoint.z) * 10, 3)]})
result["bodies"] = bodies
try:
    result["timeline"] = [design.timeline.item(i).name for i in range(design.timeline.count)]
except Exception as e:
    result["timeline"] = str(e)[:80]
try:
    r = root.activeSheetMetalRule
    result["rule"] = {"name": r.name, "thickness": r.thickness.expression, "kFactor": str(r.kFactor.expression if hasattr(r.kFactor, "expression") else r.kFactor),
                      "bendRadius": r.bendRadius.expression}
except Exception as e:
    result["rule"] = str(e)[:150]
try:
    fp = root.flatPattern
    if fp:
        bb = fp.bodies.item(0).boundingBox
        result["flat"] = [round((bb.maxPoint.x - bb.minPoint.x) * 10, 3), round((bb.maxPoint.y - bb.minPoint.y) * 10, 3),
                          round((bb.maxPoint.z - bb.minPoint.z) * 10, 3)]
    else:
        result["flat"] = None
except Exception as e:
    result["flat"] = str(e)[:150]
result["cmd"] = ui.activeCommand

