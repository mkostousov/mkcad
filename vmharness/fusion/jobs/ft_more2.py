# MKProbe job: hole, draft, sweep, loft vs analytic volumes + simple failure cases (spec M FT-10…).
import math
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
FO = adsk.fusion.FeatureOperations
rows = []


def fresh():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
    des = adsk.fusion.Design.cast(app.activeProduct)
    des.designType = adsk.fusion.DesignTypes.ParametricDesignType
    return des, des.rootComponent


def plate(root, a=4, h="10 mm"):
    sk = root.sketches.add(root.xYConstructionPlane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(a, a, 0))
    return root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI(h), FO.NewBodyFeatureOperation).bodies.item(0)


def top(body):
    return max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.centroid.z)


def case(label, expected, build):
    des, root = fresh()
    row = {"case": label, "expected_mm3": None if expected is None else round(expected, 1)}
    try:
        f = build(root)
        row["ok"] = True
        row["health"] = f.healthState
        try:
            if f.errorOrWarningMessage:
                row["msg"] = f.errorOrWarningMessage[:160]
        except Exception:
            pass
    except Exception as e:
        row["ok"] = False
        row["error"] = str(e).strip()[:200]
    b = root.bRepBodies
    row["vols_mm3"] = [round(b.item(i).volume * 1000, 1) for i in range(b.count)]
    rows.append(row)


def hole_cbore(root):
    body = plate(root)
    hi = root.features.holeFeatures.createCounterboreInput(VI("6 mm"), VI("12 mm"), VI("3 mm"))
    hi.setPositionByPoint(top(body), P3(2, 2, 1))
    hi.setDistanceExtent(VI("10 mm"))
    return root.features.holeFeatures.add(hi)
case("hole counterbore d6 depth 10 (= plate), cb d12 x3", None, hole_cbore)


def draft(deg):
    def b(root):
        body = plate(root)
        sides = [body.faces.item(i) for i in range(body.faces.count) if abs(body.faces.item(i).centroid.z - 0.5) < 1e-6]
        di = root.features.draftFeatures.createInput(sides, root.xYConstructionPlane, True)
        di.setSingleAngle(False, VI(f"{deg} deg"))
        return root.features.draftFeatures.add(di)
    return b
case("draft 4 sides 5 deg, pull plane XY", None, draft(5))
case("draft 4 sides 70 deg (sides cross)", None, draft(70))
result["rows"] = rows
