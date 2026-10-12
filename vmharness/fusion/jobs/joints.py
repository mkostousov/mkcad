# MKProbe job: assembly joints between two component occurrences (spec A.3 later phase, G).
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


def comp_box(name, x):
    m = adsk.core.Matrix3D.create(); m.translation = adsk.core.Vector3D.create(x, 0, 0)
    occ = root.occurrences.addNewComponent(m)
    c = occ.component; c.name = name
    s = c.sketches.add(c.xYConstructionPlane)
    s.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(2, 2, 0))
    c.features.extrudeFeatures.addSimple(s.profiles.item(0), VI("10 mm"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
    return occ


a = comp_box("Base", 0)
b = comp_box("Lid", 6)


def pos(o):
    t = o.transform2.translation
    return [round(t.x * 10, 3), round(t.y * 10, 3), round(t.z * 10, 3)]


def face(occ, pred):
    body = occ.bRepBodies.item(0)
    return next(body.faces.item(i) for i in range(body.faces.count) if pred(body.faces.item(i).centroid))


rows.append({"step": "start", "Base": pos(a), "Lid": pos(b), "grounded": [a.isGrounded, b.isGrounded]})
a.isGrounded = True
top_a = face(a, lambda c: abs(c.z - 1) < 1e-6)
bot_b = face(b, lambda c: abs(c.z) < 1e-6)
g0 = adsk.fusion.JointGeometry.createByPlanarFace(bot_b, None, adsk.fusion.JointKeyPointTypes.CenterKeyPoint)
g1 = adsk.fusion.JointGeometry.createByPlanarFace(top_a, None, adsk.fusion.JointKeyPointTypes.CenterKeyPoint)
ji = root.joints.createInput(g0, g1)
ji.setAsRevoluteJointMotion(adsk.fusion.JointDirections.ZAxisJointDirection)
j = root.joints.add(ji)
rows.append({"step": "revolute joint Lid bottom-centre -> Base top-centre", "Base": pos(a), "Lid": pos(b),
             "jointType": str(j.jointMotion.jointType), "health": j.healthState})
try:
    j.jointMotion.rotationValue = 3.14159265358979 / 2
    rows.append({"step": "drive rotation 90 deg", "Lid": pos(b), "rot_deg": round(j.jointMotion.rotationValue * 57.2957795, 3)})
except Exception as e:
    rows.append({"step": "drive rotation", "error": str(e)[:150]})
try:
    lim = j.jointMotion.rotationLimits
    lim.isMaximumValueEnabled = True; lim.maximumValue = 3.14159265358979 / 4
    j.jointMotion.rotationValue = 3.14159265358979
    rows.append({"step": "max limit 45 deg, request 180", "rot_deg": round(j.jointMotion.rotationValue * 57.2957795, 3)})
except Exception as e:
    rows.append({"step": "limits", "error": str(e)[:150]})
try:
    b.transform2 = adsk.core.Matrix3D.create()
    rows.append({"step": "try to move jointed Lid to origin", "Lid": pos(b)})
except Exception as e:
    rows.append({"step": "move jointed Lid", "error": str(e)[:150]})
rows.append({"timeline": [design.timeline.item(i).name for i in range(design.timeline.count)]})
result["rows"] = rows
