# MKProbe job: preselect for sheet-metal GUI commands. MODE "edge": long top edge at y=0; "face": top face (largest, highest).
MODE = "edge"
design = adsk.fusion.Design.cast(app.activeProduct)
body = design.rootComponent.bRepBodies.item(0)
ui.activeSelections.clear()
if MODE == "edge":
    zmax = max(body.vertices.item(i).geometry.z for i in range(body.vertices.count))
    e = max((body.edges.item(i) for i in range(body.edges.count)
             if all(abs(v.geometry.y) < 1e-6 and abs(v.geometry.z - zmax) < 1e-6
                    for v in (body.edges.item(i).startVertex, body.edges.item(i).endVertex))), key=lambda e: e.length)
    ui.activeSelections.add(e)
    result["edge_mm"] = round(e.length * 10, 3)
else:
    f = max((body.faces.item(i) for i in range(body.faces.count)),
            key=lambda f: (round(f.area, 6), f.centroid.z))
    ui.activeSelections.add(f)
    result["face_area_mm2"] = round(f.area * 100, 2)
result["sel"] = ui.activeSelections.count

