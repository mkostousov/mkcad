# MKProbe job: decision N.2-3 check — constraints between pieces after Trim (middle span) and Break.
P3 = adsk.core.Point3D.create
for d in [app.documents.item(i) for i in range(app.documents.count)]:
    try:
        if d.isValid and not d.isSaved:
            d.close(False)
    except Exception:
        pass
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
root = adsk.fusion.Design.cast(app.activeProduct).rootComponent
rows = []


def state(sk, label):
    lines = [sk.sketchCurves.sketchLines.item(i) for i in range(sk.sketchCurves.sketchLines.count)]
    cons = []
    for i in range(sk.geometricConstraints.count):
        g = sk.geometricConstraints.item(i)
        cons.append(g.objectType.split("::")[-1].replace("Constraint", ""))
    rows.append({"step": label,
                 "lines": [f"({round(l.startSketchPoint.geometry.x*10,2)},{round(l.startSketchPoint.geometry.y*10,2)})-"
                           f"({round(l.endSketchPoint.geometry.x*10,2)},{round(l.endSketchPoint.geometry.y*10,2)})" for l in lines],
                 "constraints": cons})


for mode in ("trim", "break"):
    sk = root.sketches.add(root.xYConstructionPlane)
    L = sk.sketchCurves.sketchLines
    h = L.addByTwoPoints(P3(0, 0, 0), P3(4, 0, 0))
    L.addByTwoPoints(P3(1, -1, 0), P3(1, 1, 0))
    L.addByTwoPoints(P3(3, -1, 0), P3(3, 1, 0))
    state(sk, f"{mode}: before")
    try:
        if mode == "trim":
            h.trim(P3(2, 0, 0))            # remove the middle span between the two verticals
        else:
            h.breakCurve(P3(2, 0, 0))      # break at the intersections nearest to the pick point
        state(sk, f"{mode}: after")
    except Exception as e:
        rows.append({"step": f"{mode}: error", "error": str(e)[:160]})
result["rows"] = rows

