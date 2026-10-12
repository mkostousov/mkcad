# MKProbe job: X-16 recovery prep — make an unsaved change (Len 60 mm) in the open saved design; report autosave prefs.
doc = app.activeDocument
des = adsk.fusion.Design.cast(doc.products.itemByProductType("DesignProductType"))
des.userParameters.itemByName("Len").expression = "60 mm"
result["doc"] = {"name": doc.name, "isModified": doc.isModified, "version": doc.dataFile.versionNumber}
gp = app.preferences.generalPreferences
result["autosave_attrs"] = {k: str(getattr(gp, k)) for k in dir(gp) if "uto" in k or "ecover" in k}
result["pid_hint"] = "kill Fusion360.exe after autosave interval"
