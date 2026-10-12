# MKProbe job: sheet metal basics — rules, flange from a sketch profile, flat pattern (spec A.3 later phase).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
rows = {}
try:
    rules = design.librarySheetMetalRules if hasattr(design, "librarySheetMetalRules") else None
    dr = design.designSheetMetalRules
    rows["design_rules"] = [dr.item(i).name for i in range(dr.count)]
    act = root.activeSheetMetalRule
    rows["active_rule"] = {"name": act.name}
    for k in ("thickness", "kFactor", "bendRadius", "reliefWidth", "reliefDepth", "reliefShape", "miterGap"):
        try:
            v = getattr(act, k)
            rows["active_rule"][k] = v.expression if hasattr(v, "expression") else str(v)
        except Exception as e:
            rows["active_rule"][k] = f"<{type(e).__name__}>"
except Exception as e:
    rows["rules_error"] = str(e)[:200]

sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(10, 5, 0))
try:
    fi = root.features.flangeFeatures.createInput(sk.profiles.item(0)) if hasattr(root.features, "flangeFeatures") else None
    if fi is None:
        raise RuntimeError("no flangeFeatures collection")
    fl = root.features.flangeFeatures.add(fi)
    b = root.bRepBodies.item(0)
    rows["flange"] = {"ok": True, "isSheetMetal": b.isSheetMetal if hasattr(b, "isSheetMetal") else None,
                      "vol_mm3": round(b.volume * 1000, 2), "faces": b.faces.count}
except Exception as e:
    rows["flange"] = {"ok": False, "error": str(e)[:200]}
methods = [m for m in dir(root.features) if "lange" in m or "heet" in m or "Flat" in m or "nfold" in m or "Bend" in m]
rows["feature_collections"] = methods
try:
    fp = root.flatPattern
    rows["flatPattern_before"] = None if fp is None else "exists"
    body = root.bRepBodies.item(0)
    top = max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.area)
    fp = root.createFlatPattern(top)
    fb = fp.bodies.item(0) if fp.bodies.count else None
    bb = fb.boundingBox if fb else None
    rows["flatPattern"] = {"ok": True, "bodies": fp.bodies.count,
                           "bbox_mm": None if not bb else [round((bb.maxPoint.x - bb.minPoint.x) * 10, 3),
                                                           round((bb.maxPoint.y - bb.minPoint.y) * 10, 3),
                                                           round((bb.maxPoint.z - bb.minPoint.z) * 10, 3)]}
except Exception as e:
    rows["flatPattern"] = {"ok": False, "error": str(e)[:200]}
rows["timeline"] = [design.timeline.item(i).name for i in range(design.timeline.count)]
result.update(rows)
