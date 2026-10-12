# MKProbe job: constraint conformance matrix (spec §4.3/§4.8).
# For each constraint: minimal geometry, apply, record success/error, fully-constrained flags and resulting geometry;
# then apply the same constraint again (redundancy) and an opposing one (conflict) where meaningful.
P3 = adsk.core.Point3D.create
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
rows = []


def r(v):
    return round(v * 10, 4)


def try_(fn):
    try:
        fn()
        return "ok"
    except Exception as e:
        return str(e).strip().split(" - ")[0].replace("3 : ", "").replace("failed to create offset: ", "")[:60]


def geo(curve):
    t = curve.objectType.split("::")[-1]
    if t == "SketchLine":
        s, e = curve.startSketchPoint.geometry, curve.endSketchPoint.geometry
        return f"L({r(s.x)},{r(s.y)})-({r(e.x)},{r(e.y)})"
    if t in ("SketchCircle", "SketchArc"):
        c = curve.centerSketchPoint.geometry
        return f"{t[6:]}(c={r(c.x)},{r(c.y)} r={r(curve.radius)})"
    return t


def case(name, build, apply, again=None, conflict=None):
    sk = root.sketches.add(root.xYConstructionPlane)
    ents = build(sk)
    row = {"constraint": name, "apply": try_(lambda: apply(sk, ents))}
    row["after"] = [geo(c) for c in [sk.sketchCurves.item(i) for i in range(sk.sketchCurves.count)]]
    row["fullyCurves"] = sum(1 for i in range(sk.sketchCurves.count) if sk.sketchCurves.item(i).isFullyConstrained)
    if again:
        row["redundant"] = try_(lambda: again(sk, ents))
    if conflict:
        row["conflict"] = try_(lambda: conflict(sk, ents))
    row["constraints"] = sk.geometricConstraints.count
    rows.append(row)
    sk.deleteMe()


L2 = lambda sk, a, b: sk.sketchCurves.sketchLines.addByTwoPoints(P3(*a, 0), P3(*b, 0))
two_lines = lambda sk: (L2(sk, (0, 0), (3, 0.4)), L2(sk, (0, 1), (2.5, 2)))
gc = lambda sk: sk.geometricConstraints

case("Horizontal", lambda sk: (L2(sk, (0, 0), (3, 0.5)),), lambda sk, e: gc(sk).addHorizontal(e[0]),
     lambda sk, e: gc(sk).addHorizontal(e[0]), lambda sk, e: gc(sk).addVertical(e[0]))
case("Vertical", lambda sk: (L2(sk, (0, 0), (0.5, 3)),), lambda sk, e: gc(sk).addVertical(e[0]),
     lambda sk, e: gc(sk).addVertical(e[0]), lambda sk, e: gc(sk).addHorizontal(e[0]))
case("Parallel", two_lines, lambda sk, e: gc(sk).addParallel(e[0], e[1]),
     lambda sk, e: gc(sk).addParallel(e[1], e[0]), lambda sk, e: gc(sk).addPerpendicular(e[0], e[1]))
case("Perpendicular", two_lines, lambda sk, e: gc(sk).addPerpendicular(e[0], e[1]),
     lambda sk, e: gc(sk).addPerpendicular(e[1], e[0]), lambda sk, e: gc(sk).addParallel(e[0], e[1]))
case("Collinear", two_lines, lambda sk, e: gc(sk).addCollinear(e[0], e[1]),
     lambda sk, e: gc(sk).addCollinear(e[1], e[0]), lambda sk, e: gc(sk).addPerpendicular(e[0], e[1]))
case("Equal (lines)", two_lines, lambda sk, e: gc(sk).addEqual(e[0], e[1]), lambda sk, e: gc(sk).addEqual(e[1], e[0]))
case("Coincident (pt-pt)", two_lines, lambda sk, e: gc(sk).addCoincident(e[0].endSketchPoint, e[1].startSketchPoint),
     lambda sk, e: gc(sk).addCoincident(e[0].endSketchPoint, e[1].startSketchPoint))
case("Coincident (pt-on-line)", two_lines, lambda sk, e: gc(sk).addCoincident(e[1].startSketchPoint, e[0]))
case("MidPoint", two_lines, lambda sk, e: gc(sk).addMidPoint(e[1].startSketchPoint, e[0]),
     lambda sk, e: gc(sk).addMidPoint(e[1].startSketchPoint, e[0]))
circles = lambda sk: (sk.sketchCurves.sketchCircles.addByCenterRadius(P3(0, 0, 0), 1),
                      sk.sketchCurves.sketchCircles.addByCenterRadius(P3(0.3, 0.2, 0), 0.5))
case("Concentric", circles, lambda sk, e: gc(sk).addConcentric(e[0], e[1]), lambda sk, e: gc(sk).addConcentric(e[1], e[0]))
case("Equal (circles)", circles, lambda sk, e: gc(sk).addEqual(e[0], e[1]))
case("Tangent (line-circle)", lambda sk: (sk.sketchCurves.sketchCircles.addByCenterRadius(P3(0, 0, 0), 1), L2(sk, (-2, 1.3), (2, 1.5))),
     lambda sk, e: gc(sk).addTangent(e[0], e[1]), lambda sk, e: gc(sk).addTangent(e[0], e[1]))
case("Fix", lambda sk: (L2(sk, (0, 0), (3, 0.5)),), lambda sk, e: setattr(e[0], "isFixed", True),
     lambda sk, e: setattr(e[0], "isFixed", True), lambda sk, e: gc(sk).addHorizontal(e[0]))
case("Symmetry", lambda sk: (L2(sk, (-1, 0.2), (-0.2, 1)), L2(sk, (0.5, 0.1), (1.4, 1.2)), L2(sk, (0, -1), (0, 2))),
     lambda sk, e: gc(sk).addSymmetry(e[0], e[1], e[2]))
result["rows"] = rows
