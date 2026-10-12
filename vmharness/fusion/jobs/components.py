# MKProbe job: components & occurrences basics (spec G.1, A.3 FPR).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent
rows = []


def tree():
    out = []
    for i in range(root.allOccurrences.count):
        o = root.allOccurrences.item(i)
        t = o.transform2.translation
        out.append({"occ": o.name, "comp": o.component.name, "bodies": o.component.bRepBodies.count,
                    "at_mm": [round(t.x * 10, 2), round(t.y * 10, 2), round(t.z * 10, 2)],
                    "vols": [round(o.bRepBodies.item(k).volume * 1000, 1) for k in range(o.bRepBodies.count)]})
    return out


def snap(label, **kw):
    r = {"step": label, "rootBodies": root.bRepBodies.count, "occurrences": tree(),
         "timeline": [design.timeline.item(i).name for i in range(design.timeline.count)]}
    r.update(kw)
    rows.append(r)


occ = root.occurrences.addNewComponent(adsk.core.Matrix3D.create())
comp = occ.component
comp.name = "Block"
sk = comp.sketches.add(comp.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(2, 2, 0))
ext = comp.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
snap("new component 'Block' with 20x20x10 body")
m = adsk.core.Matrix3D.create()
m.translation = adsk.core.Vector3D.create(5, 0, 0)
occ2 = root.occurrences.addExistingComponent(comp, m)
snap("second occurrence of same component at x=50 mm")
ext.extentOne.distance.expression = "30 mm"
snap("edit extrude in component -> 30 mm", note="both occurrences should change")
copy = root.occurrences.addNewComponentCopy(comp, adsk.core.Matrix3D.create()) if hasattr(root.occurrences, "addNewComponentCopy") else None
if copy:
    copy.component.features.extrudeFeatures.item(0).extentOne.distance.expression = "5 mm"
    snap("independent copy (addNewComponentCopy) edited to 5 mm")
# body created in root, then moved into a new component
s2 = root.sketches.add(root.xYConstructionPlane)
s2.sketchCurves.sketchCircles.addByCenterRadius(P3(-3, 0, 0), 0.5)
b = root.features.extrudeFeatures.addSimple(s2.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation).bodies.item(0)
try:
    newb = b.createComponent()
    snap("root body -> createComponent()", newBody=newb.name)
except Exception as e:
    snap("root body -> createComponent() failed", error=str(e)[:150])
result["rows"] = rows
