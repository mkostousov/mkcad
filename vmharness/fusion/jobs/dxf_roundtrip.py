# MKProbe job: refinement 1 — export a known sketch to DXF with Fusion itself, re-import it (API), describe result.
import os
P3 = adsk.core.Point3D.create
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
sk.sketchCurves.sketchCircles.addByCenterRadius(P3(2, 1, 0), 0.5)
sk.sketchCurves.sketchArcs.addByThreePoints(P3(5, 0, 0), P3(6, 1, 0), P3(7, 0, 0))
path = r"C:\mkprobe\exports\fusion_sketch.dxf"
os.makedirs(os.path.dirname(path), exist_ok=True)
result["export_ok"] = sk.saveAsDXF(path)
result["export_bytes"] = os.path.getsize(path) if os.path.exists(path) else None
with open(path, "r", errors="replace") as f:
    txt = f.read()
result["dxf_version"] = txt.split("$ACADVER")[1].split("\n")[2].strip() if "$ACADVER" in txt else None
result["dxf_insunits"] = txt.split("$INSUNITS")[1].split("\n")[2].strip() if "$INSUNITS" in txt else None
result["entity_types"] = sorted({ln.strip() for ln in txt.split("ENTITIES")[1].split("\n")[1::2]
                                 if ln.strip().isalpha() and ln.strip().isupper()}) if "ENTITIES" in txt else None


def describe(s):
    cs, gc = {}, {}
    for i in range(s.sketchCurves.count):
        t = s.sketchCurves.item(i).objectType.split("::")[-1]; cs[t] = cs.get(t, 0) + 1
    for i in range(s.geometricConstraints.count):
        t = s.geometricConstraints.item(i).objectType.split("::")[-1]; gc[t] = gc.get(t, 0) + 1
    bb = s.boundingBox
    return {"name": s.name, "curves": cs, "constraints": gc, "dims": s.sketchDimensions.count, "profiles": s.profiles.count,
            "fully": s.isFullyConstrained, "fixed": sum(1 for i in range(s.sketchCurves.count) if s.sketchCurves.item(i).isFixed),
            "bbox_mm": [round(bb.minPoint.x * 10, 3), round(bb.minPoint.y * 10, 3), round(bb.maxPoint.x * 10, 3), round(bb.maxPoint.y * 10, 3)]}


im = app.importManager
attempts = []
for label, target, plane in (("importToTarget(root), plane XY", root, root.xYConstructionPlane),
                             ("importToTarget(root), plane XZ", root, root.xZConstructionPlane)):
    before = root.sketches.count
    try:
        o = im.createDXF2DImportOptions(path, plane)
        ok = im.importToTarget(o, target)
        new = [describe(root.sketches.item(i)) for i in range(before, root.sketches.count)]
        attempts.append({"how": label, "ok": ok, "new_sketches": new})
    except Exception as e:
        attempts.append({"how": label, "error": str(e)[:200]})
result["imports"] = attempts
result["original"] = describe(sk)
