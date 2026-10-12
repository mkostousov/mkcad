# MKProbe job: keep only the unsaved document whose Sketch1 has a driven dimension (the GUI test doc); activate it.
keep = None
for i in range(app.documents.count):
    d = app.documents.item(i)
    try:
        des = adsk.fusion.Design.cast(d.products.itemByProductType("DesignProductType"))
        sks = des.rootComponent.sketches
        if sks.count == 1 and any(not sks.item(0).sketchDimensions.item(k).isDriving
                                  for k in range(sks.item(0).sketchDimensions.count)):
            keep = d
    except Exception:
        pass
closed = []
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if d != keep and not d.isSaved:
        closed.append(d.name)
        d.close(False)
if keep:
    keep.activate()
result["kept"] = keep.name if keep else None
result["closed"] = len(closed)
des = adsk.fusion.Design.cast(app.activeProduct)
result["activeEdit"] = app.activeEditObject.objectType
result["timeline"] = des.timeline.count
