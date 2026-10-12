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


def hole_simple(root):
    body = plate(root)
    hi = root.features.holeFeatures.createSimpleInput(VI("10 mm"))
    hi.setPositionByPoint(top(body), P3(2, 2, 1))
    hi.setDistanceExtent(VI("5 mm"))
    return root.features.holeFeatures.add(hi)
cone = math.pi * 25 * 5 * math.tan(math.radians(31)) / 3   # 118° drill point: cone height = r / tan(59°)
case("hole simple d10 depth 5 (+118° tip?)", 16000 - math.pi * 25 * 5, hole_simple)


def hole_cbore(root):
    body = plate(root)
    hi = root.features.holeFeatures.createCounterboreInput(VI("6 mm"), VI("12 mm"), VI("3 mm"))
    hi.setPositionByPoint(top(body), P3(2, 2, 1))
    hi.setAllExtent(adsk.fusion.ExtentDirections.NegativeExtentDirection)
    return root.features.holeFeatures.add(hi)
case("hole counterbore d6 through, cb d12 x3", 16000 - math.pi * 9 * 7 - math.pi * 36 * 3, hole_cbore)


def draft(deg):
    def b(root):
        body = plate(root)
        sides = adsk.core.ObjectCollection.create()
        for i in range(body.faces.count):
            f = body.faces.item(i)
            if abs(f.centroid.z - 0.5) < 1e-6:
                sides.add(f)
        di = root.features.draftFeatures.createInput(sides, root.xYConstructionPlane, True)
        di.setSingleAngle(False, VI(f"{deg} deg"))
        return root.features.draftFeatures.add(di)
    return b
case("draft 4 sides 5 deg, pull plane XY (bottom fixed)", None, draft(5))
case("draft 4 sides 70 deg (sides cross)", None, draft(70))


def sweep(touch=True):
    def b(root):
        skp = root.sketches.add(root.xZConstructionPlane)
        line = skp.sketchCurves.sketchLines.addByTwoPoints(skp.modelToSketchSpace(P3(0, 0, 0)), skp.modelToSketchSpace(P3(0, 0, 3)))
        skc = root.sketches.add(root.xYConstructionPlane)
        skc.sketchCurves.sketchCircles.addByCenterRadius(P3(0 if touch else 5, 0, 0), 0.5)
        path = root.features.createPath(line)
        si = root.features.sweepFeatures.createInput(skc.profiles.item(0), path, FO.NewBodyFeatureOperation)
        return root.features.sweepFeatures.add(si)
    return b
case("sweep circle r5 along 30 mm line (perpendicular)", math.pi * 25 * 30, sweep(True))
case("sweep, profile 50 mm away from path", math.pi * 25 * 30, sweep(False))


def loft(root):
    s1 = root.sketches.add(root.xYConstructionPlane)
    s1.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(2, 2, 0))
    pin = root.constructionPlanes.createInput(); pin.setByOffset(root.xYConstructionPlane, VI("20 mm"))
    s2 = root.sketches.add(root.constructionPlanes.add(pin))
    s2.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(2, 2, 0))
    li = root.features.loftFeatures.createInput(FO.NewBodyFeatureOperation)
    li.loftSections.add(s1.profiles.item(0)); li.loftSections.add(s2.profiles.item(0))
    return root.features.loftFeatures.add(li)
case("loft two equal 20x20 squares 20 mm apart", 20 * 20 * 20, loft)


def loft_one(root):
    s1 = root.sketches.add(root.xYConstructionPlane)
    s1.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(2, 2, 0))
    li = root.features.loftFeatures.createInput(FO.NewBodyFeatureOperation)
    li.loftSections.add(s1.profiles.item(0))
    return root.features.loftFeatures.add(li)
case("loft with one section", None, loft_one)
result["rows"] = rows
