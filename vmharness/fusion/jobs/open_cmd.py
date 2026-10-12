# MKProbe job template: execute a command definition by id (opens its dialog). CMD_ID substituted by tools/dialogs.ps1.
CMD_ID = "__CMD__"
ui.activeSelections.clear()
cd = ui.commandDefinitions.itemById(CMD_ID)
result["id"] = CMD_ID
result["found"] = cd is not None
if cd:
    result["name"] = cd.name
    cd.execute()
