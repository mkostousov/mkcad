# MKProbe job: X-15 2D — import DXF and SVG (40x20 mm rectangle + d10 circle) into sketches; report entities.
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
im = app.importManager
rows = []


def describe(sk):
    cs = {}
    for i in range(sk.sketchCurves.count):
        t = sk.sketchCurves.item(i).objectType.split("::")[-1]
        cs[t] = cs.get(t, 0) + 1
    gc = {}
    for i in range(sk.geometricConstraints.count):
        t = sk.geometricConstraints.item(i).objectType.split("::")[-1]
        gc[t] = gc.get(t, 0) + 1
    bb = sk.boundingBox
    return {"sketch": sk.name, "curves": cs, "constraints": gc, "dims": sk.sketchDimensions.count,
            "profiles": sk.profiles.count, "points": sk.sketchPoints.count,
            "bbox_mm": [round(bb.minPoint.x * 10, 3), round(bb.minPoint.y * 10, 3),
                        round(bb.maxPoint.x * 10, 3), round(bb.maxPoint.y * 10, 3)],
            "fully": sk.isFullyConstrained}


before = root.sketches.count
try:
    o = im.createDXF2DImportOptions(r"C:\mkprobe\rect_circle.dxf", root.xYConstructionPlane)
    im.importToTarget(o, root)
    for i in range(before, root.sketches.count):
        rows.append({"file": "dxf", **describe(root.sketches.item(i))})
    rows.append({"file": "dxf", "timeline": [design.timeline.item(i).name for i in range(design.timeline.count)]})
except Exception as e:
    rows.append({"file": "dxf", "error": str(e)[:200]})
sk = root.sketches.add(root.xZConstructionPlane)
try:
    o = im.createSVGImportOptions(r"C:\mkprobe\rect_circle.svg")
    im.importToTarget(o, sk)
    rows.append({"file": "svg (into new sketch on XZ)", **describe(sk)})
except Exception as e:
    rows.append({"file": "svg", "error": str(e)[:200]})
result["rows"] = rows
