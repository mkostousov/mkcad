# MKProbe job (X-15): import STEP / SMT / STL / OBJ / 3MF / F3D (files from export_formats.py) into new documents
# via importManager.importToTarget (into the root of a fresh design) and importToNewDocument where available.
# Record: bodies/meshes, timeline items, design type, units, body volume, solid/closed flags.
import os
D = "C:/mkprobe/exports/"
im = app.importManager

def close_unsaved():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)

def describe(design):
    root = design.rootComponent
    tl = design.timeline if design.designType == 1 else None
    out = {"designType": design.designType, "units": design.unitsManager.defaultLengthUnits,
           "bodies": [], "meshes": root.meshBodies.count, "occurrences": root.occurrences.count,
           "timeline": [(tl.item(i).name, tl.item(i).entity.objectType.split("::")[-1] if tl.item(i).entity else None)
                        for i in range(tl.count)] if design.designType == 1 else None}
    comps = [root] + [root.occurrences.item(i).component for i in range(root.occurrences.count)]
    for c in comps:
        for i in range(c.bRepBodies.count):
            b = c.bRepBodies.item(i)
            out["bodies"].append({"comp": c.name, "name": b.name, "solid": b.isSolid, "vol_mm3": round(b.volume * 1000, 3),
                                  "faces": b.faces.count})
        for i in range(c.meshBodies.count):
            m = c.meshBodies.item(i)
            out.setdefault("mesh", []).append({"comp": c.name, "name": m.name, "tris": m.displayMesh.triangleCount if m.displayMesh else None})
    return out

def opts(ext, path):
    if ext == "step":
        return im.createSTEPImportOptions(path)
    if ext == "smt":
        return im.createSMTImportOptions(path)
    if ext == "f3d":
        return im.createFusionArchiveImportOptions(path)
    if ext in ("stl", "obj", "3mf"):
        return im.createMeshImportOptions(path) if hasattr(im, "createMeshImportOptions") else None
    return None

close_unsaved()
result["im_methods"] = [m for m in dir(im) if m.startswith("create") or m.startswith("import")]
for ext in ("step", "f3d"):
    path = D + "box." + ext
    for mode, ptype in (("param", 1), ("direct", 0)):
        tag = "%s_%s" % (ext, mode)
        try:
            app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
            design = adsk.fusion.Design.cast(app.activeProduct)
            design.designType = ptype
            o = opts(ext, path)
            if o is None:
                result[tag] = {"error": "no options factory"}
            else:
                ok = im.importToTarget(o, design.rootComponent)
                r = describe(design)
                r["ok"] = bool(ok)
                if ptype == 1:
                    tl = design.timeline
                    r["groups"] = [(g.name, g.count, [g.item(j).name for j in range(g.count)]) for g in design.timeline.timelineGroups]
                result[tag] = r
        except Exception as ex:
            result[tag] = {"error": str(ex)}
        close_unsaved()
# importToNewDocument for STEP (what the GUI "Open" path most likely resembles)
try:
    n0 = app.documents.count
    doc = im.importToNewDocument(im.createSTEPImportOptions(D + "box.step"))
    d = adsk.fusion.Design.cast(doc.products.itemByProductType("DesignProductType")) if doc else None
    result["step_newdoc"] = {"doc": doc.name if doc else None, "docs": app.documents.count - n0, **(describe(d) if d else {})}
except Exception as ex:
    result["step_newdoc"] = {"error": str(ex)}
close_unsaved()
