# MKProbe job: list files in Default Project root folder (name, extension, version, complete).
proj = next(app.data.dataProjects.item(i) for i in range(app.data.dataProjects.count)
            if app.data.dataProjects.item(i).name == "Default Project")
fs = proj.rootFolder.dataFiles
result["files"] = [{"name": fs.item(i).name, "ext": fs.item(i).fileExtension, "v": fs.item(i).versionNumber,
                    "complete": getattr(fs.item(i), "isComplete", None)} for i in range(fs.count)]
