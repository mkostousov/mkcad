# MKProbe job: application / graphics environment facts for the evidence register.
gp = app.preferences.graphicsPreferences
gen = app.preferences.generalPreferences
up = app.preferences.unitAndValuePreferences


def safe(fn):
    try:
        return str(fn())
    except Exception as e:
        return f"<err {type(e).__name__}>"


result["version"] = app.version
result["userName"] = safe(lambda: app.currentUser.userName)
result["graphics"] = {k: safe(lambda k=k: getattr(gp, k)) for k in dir(gp) if not k.startswith("_") and k[0].islower()
                      and k not in ("this", "thisown", "cast", "classType", "objectType", "isValid")}
result["general"] = {k: safe(lambda k=k: getattr(gen, k)) for k in ("userLanguage", "undoStepsCount",
                     "autoSaveFrequency", "isAutoSaveEnabled", "defaultModelingOrientation")}
result["units"] = {k: safe(lambda k=k: getattr(up, k)) for k in ("generalPrecision", "angularPrecision",
                   "linearPrecision", "isScientificNotationUsed", "degreeDisplayFormat")}
result["documents_open"] = app.documents.count
