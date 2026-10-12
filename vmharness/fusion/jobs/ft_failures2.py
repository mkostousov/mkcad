# MKProbe job: revolve / pattern / mirror limit behaviour (spec M FT-06…, H.5).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
NEW = adsk.fusion.FeatureOperations.NewBodyFeatureOperation
JOIN = adsk.fusion.FeatureOperations.JoinFeatureOperation
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


def coll(items):
    c = adsk.core.ObjectCollection.create()
    for it in items:
        c.add(it)
    return c


def record(label, des, root, fn):
    row = {"case": label}
    try:
        f = fn()
        row["ok"] = True
        try:
            row["health"] = {0: "Healthy", 1: "Warning", 2: "Error"}.get(f.healthState, f.healthState)
            if f.errorOrWarningMessage:
                row["msg"] = f.errorOrWarningMessage[:200]
        except Exception:
            pass
    except Exception as e:
        row["ok"] = False
        row["error"] = str(e).strip()[:200]
    b = root.bRepBodies
    row["bodies"] = [round(b.item(i).volume * 1000, 1) for i in range(b.count)]
    row["timeline"] = des.timeline.count
    rows.append(row)


def revolve_case(label, x0, x1):
    des, root = fresh()
    sk = root.sketches.add(root.xZConstructionPlane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(sk.modelToSketchSpace(P3(x0, 0, 0)), sk.modelToSketchSpace(P3(x1, 0, 1)))
    def go():
        ri = root.features.revolveFeatures.createInput(sk.profiles.item(0), root.zConstructionAxis, NEW)
        ri.setAngleExtent(False, VI("360 deg"))
        return root.features.revolveFeatures.add(ri)
    record(label, des, root, go)


revolve_case("revolve 360, profile x 1..2 cm (clear of axis)", 1, 2)
revolve_case("revolve 360, profile x 0..2 cm (edge ON axis)", 0, 2)
revolve_case("revolve 360, profile x -1..2 cm (CROSSES axis)", -1, 2)


def box_with(label, make):
    des, root = fresh()
    sk = root.sketches.add(root.xYConstructionPlane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(1, 1, 0))
    body = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), NEW).bodies.item(0)
    record(label, des, root, lambda: make(root, body))


def rect_pattern(spacing, qty):
    def mk(root, body):
        pi = root.features.rectangularPatternFeatures.createInput(coll([body]), root.xConstructionAxis, VI(str(qty)),
                                                                 VI(spacing), adsk.fusion.PatternDistanceType.SpacingPatternDistanceType)
        return root.features.rectangularPatternFeatures.add(pi)
    return mk


box_with("rect pattern 10mm cube, qty 3, spacing 15 mm (gaps)", rect_pattern("15 mm", 3))
box_with("rect pattern 10mm cube, qty 3, spacing 5 mm (overlap)", rect_pattern("5 mm", 3))
box_with("rect pattern qty 1", rect_pattern("15 mm", 1))


def mirror_plane(offset):
    def mk(root, body):
        if offset is None:
            plane = root.yZConstructionPlane
        else:
            pin = root.constructionPlanes.createInput()
            pin.setByOffset(root.yZConstructionPlane, VI(offset))
            plane = root.constructionPlanes.add(pin)
        mi = root.features.mirrorFeatures.createInput(coll([body]), plane)
        return root.features.mirrorFeatures.add(mi)
    return mk


box_with("mirror cube about YZ (touching face)", mirror_plane(None))
box_with("mirror cube about plane x=5 mm (cuts body)", mirror_plane("5 mm"))
result["rows"] = rows
