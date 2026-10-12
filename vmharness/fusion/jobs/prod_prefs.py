pp = app.preferences.productPreferences
out = {}
for i in range(pp.count):
    it = pp.item(i)
    attrs = {k: str(getattr(it, k)) for k in dir(it) if not k.startswith("_") and k[0].islower()
             and k not in ("this", "thisown", "cast", "classType", "objectType", "isValid", "name")}
    out[it.name] = {k: v for k, v in attrs.items() if any(s in k.lower() for s in ("proj", "standard", "angle", "drawing", "unit"))}
result["productPreferences"] = out
