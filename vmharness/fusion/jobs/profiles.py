# MKProbe job: profile detection cases (§4.7) — nested loops, overlap, duplicates, self-intersection, open chains,
# tangency gaps, tiny/huge coordinates. Each case in a fresh sketch on XY of a new untitled design.
P3 = adsk.core.Point3D.create
doc = app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
cases = []


def mm(v):
    return v / 10.0


def summarize(sk):
    profs = []
    for i in range(sk.profiles.count):
        p = sk.profiles.item(i)
        try:
            area = round(p.areaProperties().area * 100, 4)
        except Exception as ex:
            area = "err:" + str(ex)[:60]
        profs.append({"loops": p.profileLoops.count, "area_mm2": area,
                      "outer": [p.profileLoops.item(j).isOuter for j in range(p.profileLoops.count)]})
    return {"curves": sk.sketchCurves.count, "points": sk.sketchPoints.count, "profiles": profs}


def run(name, build):
    sk = root.sketches.add(root.xYConstructionPlane)
    sk.name = name
    e = {"case": name}
    try:
        build(sk)
        e.update(summarize(sk))
    except Exception as ex:
        e["error"] = str(ex)[:200]
    cases.append(e)
    return sk


def rect(sk, x0, y0, x1, y1):
    return sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(mm(x0), mm(y0), 0), P3(mm(x1), mm(y1), 0))


def circ(sk, x, y, r):
    return sk.sketchCurves.sketchCircles.addByCenterRadius(P3(mm(x), mm(y), 0), mm(r))


run("nested_rect_circle", lambda s: (rect(s, 0, 0, 40, 30), circ(s, 20, 15, 5)))
run("nested_3_levels", lambda s: (rect(s, 0, 0, 60, 60), rect(s, 10, 10, 50, 50), circ(s, 30, 30, 10)))
run("two_overlapping_circles", lambda s: (circ(s, 0, 0, 10), circ(s, 12, 0, 10)))
run("touching_circles", lambda s: (circ(s, 0, 0, 10), circ(s, 20, 0, 10)))
run("duplicate_circle", lambda s: (circ(s, 0, 0, 10), circ(s, 0, 0, 10)))
run("duplicate_rect", lambda s: (rect(s, 0, 0, 20, 10), rect(s, 0, 0, 20, 10)))


def bowtie(s):
    L = s.sketchCurves.sketchLines
    a = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), mm(20), 0))
    b = L.addByTwoPoints(a.endSketchPoint, P3(mm(20), 0, 0))
    c = L.addByTwoPoints(b.endSketchPoint, P3(0, mm(20), 0))
    L.addByTwoPoints(c.endSketchPoint, a.startSketchPoint)


run("bowtie_self_intersect", bowtie)


def gap(s, g):
    L = s.sketchCurves.sketchLines
    a = L.addByTwoPoints(P3(0, 0, 0), P3(mm(20), 0, 0))
    b = L.addByTwoPoints(a.endSketchPoint, P3(mm(20), mm(20), 0))
    c = L.addByTwoPoints(b.endSketchPoint, P3(0, mm(20), 0))
    L.addByTwoPoints(c.endSketchPoint, P3(0, mm(g), 0))  # not joined to a.start


for g in (0.0, 1e-6, 1e-4, 1e-3, 0.01, 0.1):
    run("gap_%g_mm" % g, lambda s, g=g: gap(s, g))


def crossing_lines_box(s):
    rect(s, 0, 0, 40, 40)
    L = s.sketchCurves.sketchLines
    L.addByTwoPoints(P3(mm(-10), mm(20), 0), P3(mm(50), mm(20), 0))  # crosses through, unconnected


run("rect_with_crossing_line", crossing_lines_box)


def overshoot(s):
    L = s.sketchCurves.sketchLines
    L.addByTwoPoints(P3(mm(-5), 0, 0), P3(mm(25), 0, 0))
    L.addByTwoPoints(P3(mm(20), mm(-5), 0), P3(mm(20), mm(25), 0))
    L.addByTwoPoints(P3(mm(25), mm(20), 0), P3(mm(-5), mm(20), 0))
    L.addByTwoPoints(P3(0, mm(25), 0), P3(0, mm(-5), 0))


run("tic_tac_overshoot_lines", overshoot)
run("tiny_rect_1e-3mm", lambda s: rect(s, 0, 0, 0.001, 0.001))
run("tiny_rect_1e-2mm", lambda s: rect(s, 0, 0, 0.01, 0.01))
run("huge_rect_1e5mm", lambda s: rect(s, 0, 0, 1e5, 1e5))
run("huge_rect_1e6mm", lambda s: rect(s, 0, 0, 1e6, 1e6))
run("far_rect_at_1e6mm", lambda s: rect(s, 1e6, 1e6, 1e6 + 10, 1e6 + 10))


def construction_rect(s):
    for l in rect(s, 0, 0, 20, 10):
        l.isConstruction = True


run("construction_rect", construction_rect)


def zero_len(s):
    try:
        s.sketchCurves.sketchLines.addByTwoPoints(P3(0, 0, 0), P3(0, 0, 0))
        cases_note.append("zero-length line accepted")
    except Exception as ex:
        cases_note.append("zero-length line: " + str(ex)[:150])
    try:
        circ(s, 0, 0, 0)
        cases_note.append("zero-radius circle accepted")
    except Exception as ex:
        cases_note.append("zero-radius circle: " + str(ex)[:150])


cases_note = []
run("zero_size", zero_len)
result["cases"] = cases
result["notes"] = cases_note
result["doc"] = doc.name
