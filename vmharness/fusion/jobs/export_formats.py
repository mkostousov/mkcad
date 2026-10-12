# MKProbe job: which export formats the API offers and whether each succeeds on a simple box (spec §1.2, §1.3).
import os
P3 = adsk.core.Point3D.create
out_dir = r"C:\mkprobe\exports"
os.makedirs(out_dir, exist_ok=True)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
root.features.extrudeFeatures.addSimple(sk.profiles.item(0), adsk.core.ValueInput.createByString("10 mm"),
                                        adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
em = design.exportManager
creators = [m for m in dir(em) if m.startswith("create") and m.endswith("Options")]
result["exportManager_methods"] = creators
results = {}
plan = {
    "createSTEPExportOptions": "box.step", "createIGESExportOptions": "box.igs", "createSATExportOptions": "box.sat",
    "createSMTExportOptions": "box.smt", "createFusionArchiveExportOptions": "box.f3d",
    "createSTLExportOptions": "box.stl", "createOBJExportOptions": "box.obj", "createC3MFExportOptions": "box.3mf",
    "createUSDExportOptions": "box.usdz", "createDXFFlatPatternExportOptions": None,
}
for meth, fn in plan.items():
    if not hasattr(em, meth):
        results[meth] = "not available"
        continue
    if fn is None:
        results[meth] = "available (needs flat pattern; not exercised)"
        continue
    path = os.path.join(out_dir, fn)
    try:
        if os.path.exists(path):
            os.remove(path)
        opts = getattr(em, meth)(root, path) if meth in ("createSTLExportOptions", "createOBJExportOptions",
                                                          "createC3MFExportOptions", "createUSDExportOptions") \
            else getattr(em, meth)(path, root)
        ok = em.execute(opts)
        results[meth] = {"ok": ok, "bytes": os.path.getsize(path) if os.path.exists(path) else None}
    except Exception as e:
        results[meth] = {"ok": False, "error": str(e)[:200]}
result["exports"] = results
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved and d != app.activeDocument:
        pass
