# MKProbe job: user parameters & expressions — units, functions, invalid input, circularity, deletion (spec §3, §5).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
up = design.userParameters
rows = []


def add(name, expr, unit, comment=""):
    try:
        p = up.add(name, VI.createByString(expr), unit, comment)
        rows.append({"op": f"add {name} = {expr} [{unit}]", "ok": True, "expression": p.expression,
                     "value_internal": p.value, "unit": p.unit})
        return p
    except Exception as e:
        rows.append({"op": f"add {name} = {expr} [{unit}]", "ok": False, "error": str(e).strip()[:200]})


def setx(p, expr, label=None):
    before = p.expression
    try:
        p.expression = expr
        rows.append({"op": label or f"set {p.name} = {expr}", "ok": True, "expression": p.expression, "value_internal": p.value})
    except Exception as e:
        rows.append({"op": label or f"set {p.name} = {expr}", "ok": False, "error": str(e).strip()[:200],
                     "expression_after": p.expression, "expression_before": before})


L = add("Len", "40 mm", "mm", "base length")
add("LenIn", "2 in", "mm")
add("Mix", "Len + 1 in", "mm")
add("Half", "Len / 2", "mm")
add("Ang", "30 deg", "deg")
add("AngRad", "pi / 6", "rad")
add("Count", "4", "")
add("Fn", "sqrt(Len * Len) + sin(Ang) * 1 mm", "mm")
add("Cond", "max(Len; 10 mm)", "mm")
add("Cond2", "max(Len, 10 mm)", "mm")
add("BadUnit", "40 kg", "mm")
add("Unitless", "Len / 1 mm", "")
add("W", "1 mm", "mm")
add("1abc", "1 mm", "mm")
add("with space", "1 mm", "mm")
add("Len", "5 mm", "mm")
add("Undefined", "Nope * 2", "mm")
add("Syntax", "Len +* 2", "mm")
a = add("A", "10 mm", "mm")
b = add("B", "A * 2", "mm")
if a and b:
    setx(a, "B / 2", "set A = B / 2  (circular)")
# dependent update into geometry
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
ls = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
dim = sk.sketchDimensions.addDistanceDimension(ls.item(0).startSketchPoint, ls.item(0).endSketchPoint,
                                               adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0))
dim.parameter.expression = "Half"
rows.append({"op": "dim = Half", "dim_value_mm": round(dim.parameter.value * 10, 4)})
setx(L, "100 mm")
rows.append({"op": "after Len=100", "dim_value_mm": round(dim.parameter.value * 10, 4),
             "Half_mm": round(up.itemByName("Half").value * 10, 4)})
try:
    ok = up.itemByName("Half").deleteMe()
    rows.append({"op": "delete Half (used by dimension)", "ok": ok})
except Exception as e:
    rows.append({"op": "delete Half (used by dimension)", "ok": False, "error": str(e).strip()[:200]})
try:
    ok = up.itemByName("Count").deleteMe()
    rows.append({"op": "delete Count (unused)", "ok": ok})
except Exception as e:
    rows.append({"op": "delete Count (unused)", "ok": False, "error": str(e).strip()[:200]})
mp = [design.allParameters.item(i) for i in range(design.allParameters.count)]
rows.append({"op": "model parameter names", "names": [p.name for p in mp if not up.itemByName(p.name)]})
result["rows"] = rows
