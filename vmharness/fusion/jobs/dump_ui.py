# MKProbe job: dump workspaces, toolbar tabs/panels/controls and all command definitions.
# Globals provided by MKProbe: adsk, app, ui, result, log


def safe(fn, default=None):
    try:
        return fn()
    except Exception as e:
        return default if default is not None else f"<err {type(e).__name__}>"


def cmd_def(cd):
    if cd is None:
        return None
    return {
        "id": safe(lambda: cd.id),
        "name": safe(lambda: cd.name),
        "tooltip": safe(lambda: cd.tooltip),
        "isEnabled": safe(lambda: cd.controlDefinition.isEnabled),
        "isVisible": safe(lambda: cd.controlDefinition.isVisible),
        "controlType": safe(lambda: type(cd.controlDefinition).__name__),
        "isNative": safe(lambda: cd.isNative),
    }


def control(c):
    d = {
        "id": safe(lambda: c.id),
        "type": safe(lambda: c.objectType.split("::")[-1]),
        "isVisible": safe(lambda: c.isVisible),
        "isPromoted": safe(lambda: c.isPromoted),
        "isPromotedByDefault": safe(lambda: c.isPromotedByDefault),
    }
    cd = safe(lambda: c.commandDefinition, None)
    if cd is not None and not isinstance(cd, str):
        d["command"] = cmd_def(cd)
    sub = safe(lambda: c.controls, None)
    if sub is not None and not isinstance(sub, str):
        d["name"] = safe(lambda: c.name)
        d["children"] = [control(sub.item(i)) for i in range(sub.count)]
    return d


def panel(p):
    return {
        "id": safe(lambda: p.id),
        "name": safe(lambda: p.name),
        "isVisible": safe(lambda: p.isVisible),
        "productType": safe(lambda: p.productType),
        "controls": [control(p.controls.item(i)) for i in range(p.controls.count)],
    }


workspaces = []
for i in range(ui.workspaces.count):
    ws = ui.workspaces.item(i)
    w = {
        "id": safe(lambda: ws.id),
        "name": safe(lambda: ws.name),
        "productType": safe(lambda: ws.productType),
        "isActive": safe(lambda: ws.isActive),
        "isNative": safe(lambda: ws.isNative),
        "tabs": [],
    }
    tabs = safe(lambda: ws.toolbarTabs, None)
    if tabs is not None and not isinstance(tabs, str):
        for t in range(tabs.count):
            tab = tabs.item(t)
            w["tabs"].append({
                "id": safe(lambda: tab.id),
                "name": safe(lambda: tab.name),
                "isVisible": safe(lambda: tab.isVisible),
                "panels": [panel(tab.toolbarPanels.item(k)) for k in range(tab.toolbarPanels.count)],
            })
    workspaces.append(w)

toolbars = []
for i in range(ui.toolbars.count):
    tb = ui.toolbars.item(i)
    toolbars.append({"id": safe(lambda: tb.id),
                     "controls": [control(tb.controls.item(k)) for k in range(tb.controls.count)]})

palettes = []
for i in range(ui.palettes.count):
    p = ui.palettes.item(i)
    palettes.append({"id": safe(lambda: p.id), "name": safe(lambda: p.name), "isVisible": safe(lambda: p.isVisible)})

defs = ui.commandDefinitions
result["app"] = {"version": app.version, "os": safe(lambda: app.preferences.generalPreferences.userLanguage)}
result["activeWorkspace"] = safe(lambda: ui.activeWorkspace.id)
result["workspaces"] = workspaces
result["toolbars"] = toolbars
result["palettes"] = palettes
result["commandDefinitions"] = [cmd_def(defs.item(i)) for i in range(defs.count)]
log(f"{len(workspaces)} workspaces, {defs.count} command definitions")
