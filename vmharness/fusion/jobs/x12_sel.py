# MKProbe job: report what is selected (type + compact geometry) and clear the selection.
out = []
for i in range(ui.activeSelections.count):
    e = ui.activeSelections.item(i).entity
    t = e.objectType.split("::")[-1]
    info = {"type": t}
    try:
        if t == "BRepVertex":
            g = e.geometry; info["at_mm"] = [round(g.x * 10, 2), round(g.y * 10, 2), round(g.z * 10, 2)]
        elif t == "BRepEdge":
            info["length_mm"] = round(e.length * 10, 2)
        elif t == "BRepFace":
            c = e.centroid; info["centroid_mm"] = [round(c.x * 10, 2), round(c.y * 10, 2), round(c.z * 10, 2)]
    except Exception:
        pass
    out.append(info)
result["selected"] = out
ui.activeSelections.clear()
