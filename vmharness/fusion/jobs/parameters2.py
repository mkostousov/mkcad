# MKProbe job: follow-ups — circular references, pi/rad, self reference, naming counter.
VI = adsk.core.ValueInput
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
up = design.userParameters
rows = []


def add(name, expr, unit):
    try:
        p = up.add(name, VI.createByString(expr), unit, "")
        rows.append({"op": f"add {name} = {expr} [{unit}]", "ok": True, "value": p.value})
        return p
    except Exception as e:
        rows.append({"op": f"add {name} = {expr} [{unit}]", "ok": False, "error": str(e).strip()[:200]})


for expr, unit in (("pi / 6", "rad"), ("pi / 6", ""), ("30 deg", "rad"), ("pi / 6 rad", "deg"), ("PI", "")):
    add(f"t{len(rows)}", expr, unit)
alpha = add("Alpha", "10 mm", "mm")
beta = add("Beta", "Alpha * 2", "mm")
for expr, label in (("Beta / 2", "Alpha = Beta / 2 (2-cycle)"), ("Alpha + 1 mm", "Alpha = Alpha + 1 mm (self)")):
    try:
        alpha.expression = expr
        rows.append({"op": label, "ok": True, "expression": alpha.expression})
    except Exception as e:
        rows.append({"op": label, "ok": False, "error": str(e).strip()[:200], "expression_after": alpha.expression})
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
P3 = adsk.core.Point3D.create
ls = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
d = sk.sketchDimensions.addDistanceDimension(ls.item(0).startSketchPoint, ls.item(0).endSketchPoint,
                                             adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0))
rows.append({"op": "first dimension name in fresh doc", "name": d.parameter.name})
rows.append({"op": "all parameter names", "names": [design.allParameters.item(i).name for i in range(design.allParameters.count)]})
result["rows"] = rows
