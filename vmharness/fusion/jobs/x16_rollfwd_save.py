# MKProbe job: move the timeline marker to the end and save a new version (for drawing creation).
doc = app.activeDocument
des = adsk.fusion.Design.cast(doc.products.itemByProductType("DesignProductType"))
des.timeline.moveToEnd()
ok = doc.save("v3: marker at end for drawing test")
adsk.doEvents()
result["saved"] = ok
result["marker"] = des.timeline.markerPosition
result["modified"] = doc.isModified
