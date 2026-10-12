# MKProbe job: feature failure / limit behaviour on a 40x20x10 mm box (spec M FT-*, H.5).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
NEW = adsk.fusion.FeatureOperations.NewBodyFeatureOperation
rows = []


def fresh():
    for i in reversed(range(app.documents.count)):
        d = app.documents.item(i)
        if not d.isSaved:
            d.close(False)
    app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
    des = adsk.fusion.Design.cast(app.activeProduct)
    des.designType = adsk.fusion.DesignTypes.ParametricDesignType
    root = des.rootComponent
    sk = root.sketches.add(root.xYConstructionPlane)
    sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
    body = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), NEW).bodies.item(0)
    return des, root, body


def top_face(body):
    return max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.centroid.z)


def coll(items):
    c = adsk.core.ObjectCollection.create()
    for it in items:
        c.add(it)
    return c


def run(label, make):
    des, root, body = fresh()
    row = {"case": label}
    try:
        f = make(root, body)
        row["ok"] = True
        try:
            row["health"] = {0: "Healthy", 1: "Warning", 2: "Error"}.get(f.healthState, f.healthState)
            if f.errorOrWarningMessage:
                row["msg"] = f.errorOrWarningMessage[:220]
        except Exception:
            pass
    except Exception as e:
        row["ok"] = False
        row["error"] = str(e).strip()[:220]
    b = root.bRepBodies
    row["bodies"] = [{"vol_mm3": round(b.item(i).volume * 1000, 1), "faces": b.item(i).faces.count,
                      "solid": b.item(i).isSolid} for i in range(b.count)]
    row["timeline"] = des.timeline.count
    rows.append(row)


def fillet(r, which="top"):
    def mk(root, body):
        fi = root.features.filletFeatures.createInput()
        edges = top_face(body).edges if which == "top" else body.edges
        fi.addConstantRadiusEdgeSet(coll([edges.item(i) for i in range(edges.count)]), VI(r), True)
        return root.features.filletFeatures.add(fi)
    return mk


def chamfer(d):
    def mk(root, body):
        ci = root.features.chamferFeatures.createInput2()
        e = top_face(body).edges
        ci.chamferEdgeSets.addEqualDistanceChamferEdgeSet(coll([e.item(i) for i in range(e.count)]), VI(d), True)
        return root.features.chamferFeatures.add(ci)
    return mk


def shell(t):
    def mk(root, body):
        si = root.features.shellFeatures.createInput(coll([top_face(body)]), False)
        si.insideThickness = VI(t)
        return root.features.shellFeatures.add(si)
    return mk


run("fillet top edges r2", fillet("2 mm"))
run("fillet top edges r5 (= half width... 20 mm side)", fillet("5 mm"))
run("fillet top edges r10 (= height)", fillet("10 mm"))
run("fillet top edges r12 (> height)", fillet("12 mm"))
run("fillet all edges r6", fillet("6 mm", "all"))
run("chamfer top edges 2", chamfer("2 mm"))
run("chamfer top edges 11 (> height)", chamfer("11 mm"))
run("shell open top t2", shell("2 mm"))
run("shell open top t9 (wall > half width)", shell("9 mm"))
run("shell open top t10 (= half width)", shell("10 mm"))
result["rows"] = rows
