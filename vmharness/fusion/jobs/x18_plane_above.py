# MKProbe job template: X-18 Extrude default operation. CASE is substituted by tools/x18.ps1.
CASE = "plane_above"
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput
NEW = adsk.fusion.FeatureOperations.NewBodyFeatureOperation
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
design.designType = adsk.fusion.DesignTypes.ParametricDesignType
root = design.rootComponent


def box(x0, y0, x1, y1, h="10 mm"):
    s = root.sketches.add(root.xYConstructionPlane)
    s.sketchCurves.sketchLines.addTwoPointRectangle(P3(x0, y0, 0), P3(x1, y1, 0))
    return root.features.extrudeFeatures.addSimple(s.profiles.item(0), VI.createByString(h), NEW).bodies.item(0)


b1 = box(0, 0, 4, 2)
if CASE == "two_bodies_top":
    box(6, 0, 8, 2)
top = max((b1.faces.item(i) for i in range(b1.faces.count)), key=lambda f: f.centroid.z)
if CASE in ("xy_inside", "xy_partial", "xy_touch", "xy_far"):
    sk = root.sketches.add(root.xYConstructionPlane)
    if CASE == "xy_inside":
        sk.sketchCurves.sketchCircles.addByCenterRadius(P3(1, 1, 0), 0.5)
    elif CASE == "xy_partial":
        sk.sketchCurves.sketchCircles.addByCenterRadius(P3(4, 1, 0), 0.5)
    elif CASE == "xy_touch":
        sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(4, 0, 0), P3(5, 2, 0))
    else:
        sk.sketchCurves.sketchCircles.addByCenterRadius(P3(10, 10, 0), 0.5)
elif CASE == "plane_above":
    pi = root.constructionPlanes.createInput()
    pi.setByOffset(root.xYConstructionPlane, VI.createByString("20 mm"))
    sk = root.sketches.add(root.constructionPlanes.add(pi))
    sk.sketchCurves.sketchCircles.addByCenterRadius(P3(1, 1, 0), 0.5)
elif CASE in ("top_face_outside", "two_bodies_top", "top_inside"):
    sk = root.sketches.add(top)
    c = P3(10, 10, 1) if CASE == "top_face_outside" else P3(1, 1, 1)
    sk.sketchCurves.sketchCircles.addByCenterRadius(sk.modelToSketchSpace(c), 0.5)
ui.activeSelections.clear()
# choose the profile whose area matches the circle/rectangle we drew (not regions created by projected face edges)
profs = [sk.profiles.item(i) for i in range(sk.profiles.count)]
target = min(profs, key=lambda p: abs(p.areaProperties().area - (3.14159265 * 0.25 if CASE != "xy_touch" else 2.0)))
ui.activeSelections.add(target)
result["case"] = CASE
result["profiles"] = len(profs)
result["sel_area_mm2"] = round(target.areaProperties().area * 100, 2)
result["bodies"] = root.bRepBodies.count
app.activeViewport.fit()


