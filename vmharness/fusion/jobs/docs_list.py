# MKProbe job: list open documents (name, type, saved, modified, active) and active command.
out = []
for i in range(app.documents.count):
    d = app.documents.item(i)
    out.append({"name": d.name, "type": d.objectType.split("::")[-1], "saved": d.isSaved, "modified": d.isModified,
                "active": d == app.activeDocument, "products": [d.products.item(k).productType for k in range(d.products.count)]})
result["docs"] = out
result["cmd"] = ui.activeCommand
