# MKProbe job: X-06 — make the reference line (0,0)-(40,0) of the active sketch a construction line.
sk = adsk.fusion.Sketch.cast(app.activeEditObject)
done = []
for i in range(sk.sketchCurves.sketchLines.count):
    l = sk.sketchCurves.sketchLines.item(i)
    s, e = l.startSketchPoint.geometry, l.endSketchPoint.geometry
    if abs(s.y) < 1e-6 and abs(e.y) < 1e-6 and abs(abs(e.x - s.x) - 4) < 1e-6:
        l.isConstruction = True
        done.append(i)
result["construction_lines"] = done
result["active"] = app.activeEditObject.objectType
