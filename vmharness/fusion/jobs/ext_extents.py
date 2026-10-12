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


def sym(root):
    i = ext(root, square_sketch(root)); i.setSymmetricExtent(VI("10 mm"), True)   # True: whole length = 10
    return root.features.extrudeFeatures.add(i)
case("symmetric, isFullLength=True, 10 mm, 20x20 square", 20 * 20 * 10, sym)


def sym_half(root):
    i = ext(root, square_sketch(root)); i.setSymmetricExtent(VI("10 mm"), False)  # 10 each side
    return root.features.extrudeFeatures.add(i)
case("symmetric, isFullLength=False, 10 mm", 20 * 20 * 20, sym_half)


def two(root):
    i = ext(root, square_sketch(root))
    i.setTwoSidesExtent(adsk.fusion.DistanceExtentDefinition.create(VI("10 mm")),
                        adsk.fusion.DistanceExtentDefinition.create(VI("5 mm")))
    return root.features.extrudeFeatures.add(i)
case("two sides 10 + 5 mm", 20 * 20 * 15, two)


def taper(deg):
    def b(root):
        i = ext(root, square_sketch(root))
        i.setOneSideExtent(adsk.fusion.DistanceExtentDefinition.create(VI("10 mm")),
                           adsk.fusion.ExtentDirections.PositiveExtentDirection, VI(f"{deg} deg"))
        return root.features.extrudeFeatures.add(i)
    return b
t = math.tan(math.radians(10))
frustum = lambda a, h, tt: h / 3 * (a * a + (a - 2 * h * tt) ** 2 + a * (a - 2 * h * tt))
case("taper +10 deg, 10 mm (expect inward = frustum?)", frustum(20, 10, t), taper(10))
case("taper -10 deg, 10 mm", frustum(20, 10, -t), taper(-10))
case("taper 45 deg, 10 mm (top collapses to 0)", frustum(20, 10, 1), taper(45))
case("taper 50 deg, 10 mm (self-intersects)", None, taper(50))


def through_all(root):
    sk = square_sketch(root, size=4)
    root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), FO.NewBodyFeatureOperation)
    s2 = root.sketches.add(root.xYConstructionPlane)
    s2.sketchCurves.sketchCircles.addByCenterRadius(P3(2, 2, 0), 0.5)
    i = ext(root, s2, FO.CutFeatureOperation)
    i.setAllExtent(adsk.fusion.ExtentDirections.SymmetricExtentDirection)
    return root.features.extrudeFeatures.add(i)
case("through-all cut d10 hole in 40x40x10 plate", 40 * 40 * 10 - math.pi * 25 * 10, through_all)


def to_object(root):
    pin = root.constructionPlanes.createInput()
    pin.setByOffset(root.xYConstructionPlane, VI("25 mm"))
    target = root.constructionPlanes.add(pin)
    i = ext(root, square_sketch(root))
    i.setOneSideExtent(adsk.fusion.ToEntityExtentDefinition.create(target, False),
                       adsk.fusion.ExtentDirections.PositiveExtentDirection)
    return root.features.extrudeFeatures.add(i)
case("to object: offset plane at 25 mm", 20 * 20 * 25, to_object)


def intersect(root):
    sk = square_sketch(root)
    root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), FO.NewBodyFeatureOperation)
    s2 = root.sketches.add(root.xYConstructionPlane)
    s2.sketchCurves.sketchLines.addTwoPointRectangle(P3(1, 1, 0), P3(3, 3, 0))
    i = ext(root, s2, FO.IntersectFeatureOperation)
    i.setDistanceExtent(False, VI("20 mm"))
    return root.features.extrudeFeatures.add(i)
case("intersect: 20x20x10 box with 20x20x20 at offset (10,10)", 10 * 10 * 10, intersect)
result["rows"] = rows
