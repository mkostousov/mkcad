# MKProbe job: X-16 — modify + save (version 2), close, reopen from the project, compare persisted state.
import time
doc = next(app.documents.item(i) for i in range(app.documents.count) if app.documents.item(i).name.startswith("mk_x16_persist"))
doc.activate()
design = adsk.fusion.Design.cast(doc.products.itemByProductType("DesignProductType"))
r = {}
design.userParameters.itemByName("Len").expression = "50 mm"
r["after_edit"] = {"isModified": doc.isModified, "isSaved": doc.isSaved, "name": doc.name}
ok = doc.save("v2: Len 50 mm")
adsk.doEvents()
r["save2"] = {"ok": ok, "version": doc.dataFile.versionNumber, "isModified": doc.isModified,
              "latestVersion": doc.dataFile.latestVersionNumber if hasattr(doc.dataFile, "latestVersionNumber") else None}
df = doc.dataFile
doc.close(False)
r["open_docs_after_close"] = [app.documents.item(i).name for i in range(app.documents.count)]
t0 = time.time()
doc2 = app.documents.open(df)
r["reopen_seconds"] = round(time.time() - t0, 2)
d2 = adsk.fusion.Design.cast(doc2.products.itemByProductType("DesignProductType"))
vp = app.activeViewport
cam = vp.camera
r["after_reopen"] = {"name": doc2.name, "version": doc2.dataFile.versionNumber, "isModified": doc2.isModified,
                     "marker": d2.timeline.markerPosition, "timeline": d2.timeline.count,
                     "timelineNames": [d2.timeline.item(i).name for i in range(d2.timeline.count)],
                     "Len": d2.userParameters.itemByName("Len").expression,
                     "eye": [round(cam.eye.x, 3), round(cam.eye.y, 3), round(cam.eye.z, 3)],
                     "namedViews": [d2.namedViews.item(i).name for i in range(d2.namedViews.count)],
                     "selection": ui.activeSelections.count,
                     "bodyVol_mm3": round(d2.rootComponent.bRepBodies.item(0).volume * 1000, 2)}
vs = df.versions
r["versions"] = [{"v": vs.item(i).versionNumber, "desc": vs.item(i).description} for i in range(vs.count)]
result.update(r)
