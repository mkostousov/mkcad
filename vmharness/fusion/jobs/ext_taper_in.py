# MKProbe job: extrude extent variants vs analytic volumes (spec M EXT-03…10).
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


def square_sketch(root, plane=None, size=2):
    sk = root.sketches.add(plane or root.xYConstructionPlane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(size, size, 0))
    return sk


def case(label, expected_mm3, build):
    des, root = fresh()
    row = {"case": label, "expected_mm3": None if expected_mm3 is None else round(expected_mm3, 1)}
    try:
        f = build(root)
        row["ok"] = True
        row["health"] = f.healthState
    except Exception as e:
        row["ok"] = False
        row["error"] = str(e).strip()[:200]
    b = root.bRepBodies
    row["vols_mm3"] = [round(b.item(i).volume * 1000, 1) for i in range(b.count)]
    rows.append(row)


def ext(root, sk, op=FO.NewBodyFeatureOperation):
    return root.features.extrudeFeatures.createInput(sk.profiles.item(0), op)


def taper(deg):
    def b(root):
        i = ext(root, square_sketch(root))
        i.setOneSideExtent(adsk.fusion.DistanceExtentDefinition.create(VI("10 mm")),
                           adsk.fusion.ExtentDirections.PositiveExtentDirection, VI(f"{deg} deg"))
        return root.features.extrudeFeatures.add(i)
    return b
t = math.tan(math.radians(10))
frustum = lambda a, h, tt: h / 3 * (a * a + (a - 2 * h * tt) ** 2 + a * (a - 2 * h * tt))
case("taper -30 deg, 10 mm", frustum(20, 10, -math.tan(math.radians(30))), taper(-30))
case("taper -45 deg, 10 mm (apex exactly at top)", 20*20*10/3, taper(-45))
case("taper -50 deg, 10 mm (would invert)", None, taper(-50))
result["rows"] = rows

