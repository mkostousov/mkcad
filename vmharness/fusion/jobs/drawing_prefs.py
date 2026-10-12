# MKProbe job: look for drawing preferences (projection angle, standard) in app.preferences.
pr = app.preferences
names = [n for n in dir(pr) if not n.startswith("_") and n[0].islower()]
result["preference_groups"] = names
for n in names:
    if "raw" in n.lower():
        g = getattr(pr, n)
        result[n] = {k: str(getattr(g, k)) for k in dir(g) if not k.startswith("_") and k[0].islower()
                     and k not in ("this", "thisown", "cast", "classType", "objectType", "isValid")}
