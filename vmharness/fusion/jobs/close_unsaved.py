# MKProbe job: close all unsaved documents without saving, then open one fresh untitled design.
closed = []
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        closed.append(d.name)
        d.close(False)
keep = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
result["closed"] = closed
result["open"] = [app.documents.item(i).name for i in range(app.documents.count)]
