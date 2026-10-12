# MKProbe job: export the active drawing to PDF (and DXF if offered) via the drawing ExportManager.
import os
doc = app.activeDocument
result["docType"] = doc.objectType
result["products"] = [doc.products.item(i).productType for i in range(doc.products.count)]
result["activeProduct"] = app.activeProduct.objectType
drw = adsk.drawing.Drawing.cast(app.activeProduct)
result["hasDrawing"] = drw is not None
out = {}
if drw:
    em = drw.exportManager
    out["methods"] = [m for m in dir(em) if m.startswith("create")]
    p = r"C:\mkprobe\exports\drawing.pdf"
    try:
        if os.path.exists(p):
            os.remove(p)
        o = em.createPDFExportOptions(p)
        out["pdf"] = {"ok": em.execute(o), "bytes": os.path.getsize(p) if os.path.exists(p) else None}
    except Exception as e:
        out["pdf"] = {"error": str(e)[:160]}
    if hasattr(em, "createDXFExportOptions"):
        q = r"C:\mkprobe\exports\drawing.dxf"
        try:
            o = em.createDXFExportOptions(q)
            out["dxf"] = {"ok": em.execute(o), "bytes": os.path.getsize(q) if os.path.exists(q) else None}
        except Exception as e:
            out["dxf"] = {"error": str(e)[:160]}
result["export"] = out

