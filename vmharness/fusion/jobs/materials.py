# MKProbe job: default material, mass properties, material change, appearance vs material (spec A.3, H.3.7).
P3 = adsk.core.Point3D.create
VI = adsk.core.ValueInput.createByString
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
body = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10 mm"),
                                               adsk.fusion.FeatureOperations.NewBodyFeatureOperation).bodies.item(0)
rows = []


def snap(label):
    pp = body.physicalProperties
    c = pp.centerOfMass
    rows.append({"step": label, "material": body.material.name if body.material else None,
                 "appearance": body.appearance.name if body.appearance else None,
                 "density_kg_m3": round(pp.density * 1000, 3),   # API g/cm^3 -> kg/m^3
                 "mass_g": round(pp.mass * 1000, 3), "volume_mm3": round(pp.volume * 1000, 3),
                 "area_mm2": round(pp.area * 100, 3),
                 "com_mm": [round(c.x * 10, 3), round(c.y * 10, 3), round(c.z * 10, 3)],
                 "componentMaterial": root.material.name if root.material else None})


snap("default")
libs = [app.materialLibraries.item(i).name for i in range(app.materialLibraries.count)]
result["material_libraries"] = libs
lib = app.materialLibraries.itemByName("Fusion Material Library")
alu = None
if lib:
    for i in range(lib.materials.count):
        m = lib.materials.item(i)
        if m.name.startswith("Aluminum"):
            alu = m
            break
if alu:
    body.material = alu
    snap("material -> " + alu.name)
alib = app.materialLibraries.itemByName("Fusion Appearance Library")
if alib:
    for i in range(alib.appearances.count):
        a = alib.appearances.item(i)
        if "Red" in a.name or "red" in a.name:
            body.appearance = design.appearances.itemByName(a.name) or design.appearances.addByCopy(a, a.name)
            break
    snap("appearance override set")
    if alu:
        body.material = alu
        snap("material re-applied after appearance override")
result["rows"] = rows
