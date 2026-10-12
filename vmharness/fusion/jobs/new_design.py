# MKProbe job: open an untitled (never saved) Fusion design and report visible UI state.
doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
ws = ui.activeWorkspace
result["document"] = {"name": doc.name, "isSaved": doc.isSaved, "isModified": doc.isModified}
result["activeWorkspace"] = ws.id
result["visibleTabs"] = [ws.toolbarTabs.item(i).id for i in range(ws.toolbarTabs.count)
                         if ws.toolbarTabs.item(i).isVisible]
result["activeTab"] = [ws.toolbarTabs.item(i).id for i in range(ws.toolbarTabs.count)
                       if ws.toolbarTabs.item(i).isActive]
design = adsk.fusion.Design.cast(app.activeProduct)
result["designType"] = str(design.designType)  # 0 direct, 1 parametric
result["units"] = design.unitsManager.defaultLengthUnits
