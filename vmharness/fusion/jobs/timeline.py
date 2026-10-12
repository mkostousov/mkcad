# MKProbe job: parametric history / timeline behaviour (spec §3, §5 persistent references).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput
H = adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation
Vt = adsk.fusion.DimensionOrientations.VerticalDimensionOrientation
steps = []

app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
tl = design.timeline


def r(x, n=4):
    return round(x, n)


def state(label, **extra):
    items = []
    for i in range(tl.count):
        t = tl.item(i)
        e = t.entity
        it = {"i": i, "name": t.name, "type": e.objectType.split("::")[-1] if e else None,
              "suppressed": t.isSuppressed, "rolledBack": t.isRolledBack}
        for attr in ("healthState", "errorOrWarningMessage"):
            try:
                v = getattr(e, attr)
                if attr == "healthState":
                    v = {0: "Healthy", 1: "Warning", 2: "Error", 3: "Suppressed", 4: "Unknown"}.get(v, v)
                if v not in (None, ""):
                    it[attr] = v
            except Exception:
                pass
        items.append(it)
    bodies = [{"name": b.name, "vol_cm3": r(b.volume), "faces": b.faces.count, "edges": b.edges.count}
              for b in [root.bRepBodies.item(i) for i in range(root.bRepBodies.count)]]
    s = {"step": label, "marker": tl.markerPosition, "items": items, "bodies": bodies}
    s.update(extra)
    steps.append(s)


def attempt(fn):
    try:
        out = fn()
        return {"ok": True, "returned": str(out)[:80] if out is not None and not hasattr(out, "objectType") else None}
    except Exception as e:
        return {"ok": False, "error": str(e).strip()[:300]}


# --- build: constrained 40x20 rectangle -> extrude 10 -> fillet top edges r2 -> sketch circle on top -> cut
sk = root.sketches.add(root.xYConstructionPlane)
L = sk.sketchCurves.sketchLines
a = L.addByTwoPoints(sk.originPoint, P3(4, 0, 0)); b = L.addByTwoPoints(a.endSketchPoint, P3(4, 2, 0))
c = L.addByTwoPoints(b.endSketchPoint, P3(0, 2, 0)); d = L.addByTwoPoints(c.endSketchPoint, a.startSketchPoint)
g = sk.geometricConstraints
g.addHorizontal(a); g.addHorizontal(c); g.addVertical(b); g.addVertical(d)
wdim = sk.sketchDimensions.addDistanceDimension(a.startSketchPoint, a.endSketchPoint, H, P3(2, -1, 0))
sk.sketchDimensions.addDistanceDimension(b.startSketchPoint, b.endSketchPoint, Vt, P3(5, 1, 0))
ext = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI.createByString("10 mm"),
                                              adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
body = ext.bodies.item(0)
top = max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.centroid.z)
edges = adsk.core.ObjectCollection.create()
for i in range(top.edges.count):
    edges.add(top.edges.item(i))
fi = root.features.filletFeatures.createInput()
fi.addConstantRadiusEdgeSet(edges, VI.createByString("2 mm"), False)
fillet = root.features.filletFeatures.add(fi)
top2 = max((body.faces.item(i) for i in range(body.faces.count)), key=lambda f: f.centroid.z)
sk2 = root.sketches.add(top2)
circ = sk2.sketchCurves.sketchCircles.addByCenterRadius(sk2.modelToSketchSpace(P3(1, 1, 1)), 0.4)
cut = root.features.extrudeFeatures.addSimple(sk2.profiles.item(0), VI.createByString("-5 mm"),
                                              adsk.fusion.FeatureOperations.CutFeatureOperation)
state("built: sketch, extrude 10, fillet r2 top, sketch on top face, cut d8 depth 5")

dist = ext.extentOne.distance if hasattr(ext, "extentOne") else None
state("extrude 10->1 mm (fillet r2 now larger than height)",
      res=attempt(lambda: setattr(ext.extentOne.distance, "expression", "1 mm")))
state("extrude back to 10 mm", res=attempt(lambda: setattr(ext.extentOne.distance, "expression", "10 mm")))
state("width 40->60 mm (sketch on top face must follow)", res=attempt(lambda: setattr(wdim.parameter, "expression", "60 mm")),
      circle_center_model=[r(v) for v in (lambda p: (p.x, p.y, p.z))(sk2.sketchToModelSpace(circ.centerSketchPoint.geometry))])
state("width 60->5 mm (cut circle now outside face)", res=attempt(lambda: setattr(wdim.parameter, "expression", "5 mm")))
state("width back to 40 mm", res=attempt(lambda: setattr(wdim.parameter, "expression", "40 mm")))
ft = fillet.timelineObject
state("suppress fillet", res=attempt(lambda: setattr(ft, "isSuppressed", True)))
state("unsuppress fillet", res=attempt(lambda: setattr(ft, "isSuppressed", False)))
state("marker -> after extrude (index 2)", res=attempt(lambda: tl.item(1).rollTo(False)))
state("marker -> end", res=attempt(lambda: tl.moveToEnd()))
ct = cut.timelineObject
state("reorder: cut before fillet", res=attempt(lambda: ct.reorder(ft.index)), canReorderCut=ct.canReorder(ft.index))
state("reorder: sketch-on-face before extrude (should be illegal)",
      res=attempt(lambda: sk2.timelineObject.reorder(ext.timelineObject.index)))
state("delete base extrude", res=attempt(lambda: ext.deleteMe()))
result["steps"] = steps
