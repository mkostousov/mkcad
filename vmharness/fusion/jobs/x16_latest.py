# MKProbe job: X-16 — close open copies, open the LATEST version of mk_x16_persist from Default Project, report.
for i in reversed(range(app.documents.count)):
    app.documents.item(i).close(False)
proj = next(app.data.dataProjects.item(i) for i in range(app.data.dataProjects.count)
            if app.data.dataProjects.item(i).name == "Default Project")
files = proj.rootFolder.dataFiles
df = next(files.item(i) for i in range(files.count) if files.item(i).name == "mk_x16_persist")
result["listed"] = {"name": df.name, "version": df.versionNumber, "latest": df.latestVersionNumber,
                    "isComplete": getattr(df, "isComplete", None)}
latest = df.latestVersion if hasattr(df, "latestVersion") else df
doc = app.documents.open(latest)
des = adsk.fusion.Design.cast(doc.products.itemByProductType("DesignProductType"))
result["opened"] = {"version": doc.dataFile.versionNumber, "Len": des.userParameters.itemByName("Len").expression,
                    "marker": des.timeline.markerPosition,
                    "bodyX_mm": round((des.rootComponent.bRepBodies.item(0).boundingBox.maxPoint.x -
                                       des.rootComponent.bRepBodies.item(0).boundingBox.minPoint.x) * 10, 3)}
