# MKProbe job: DXF import unit handling ($INSUNITS 4 mm / 1 inch / absent) and repeated import behaviour.
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
im = app.importManager
rows = []


def snap():
    out = []
    for i in range(root.sketches.count):
        s = root.sketches.item(i)
        bb = s.boundingBox
        out.append({"name": s.name, "plane": s.referencePlane.name if hasattr(s.referencePlane, "name") else str(s.referencePlane.objectType),
                    "curves": s.sketchCurves.count, "fixed": sum(1 for k in range(s.sketchCurves.count) if s.sketchCurves.item(k).isFixed),
                    "bbox_mm": [round((bb.maxPoint.x - bb.minPoint.x) * 10, 3), round((bb.maxPoint.y - bb.minPoint.y) * 10, 3)]})
    return out


for f, plane in (("r12.dxf", root.xYConstructionPlane), ("r12_inch.dxf", root.xYConstructionPlane),
                 ("r12_nounits.dxf", root.xYConstructionPlane), ("r12.dxf", root.xZConstructionPlane)):
    try:
        o = im.createDXF2DImportOptions("C:\\mkprobe\\" + f, plane)
        ok = im.importToTarget(o, root)
        rows.append({"file": f, "plane": plane.name, "ok": ok, "sketches": snap(),
                     "timeline": [design.timeline.item(i).name for i in range(design.timeline.count)]})
    except Exception as e:
        rows.append({"file": f, "plane": plane.name, "error": str(e)[:160]})
result["rows"] = rows
