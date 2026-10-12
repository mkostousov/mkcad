# MKProbe job: document unit change mm -> in -> mm: expressions, values, new-input interpretation (spec M UN-01…04).
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
um = design.fusionUnitsManager
up = design.userParameters
up.add("Len", VI("40 mm"), "mm", "")
up.add("Wid", VI("1 in"), "in", "")
sk = root.sketches.add(root.xYConstructionPlane)
ls = sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
d = sk.sketchDimensions.addDistanceDimension(ls.item(0).startSketchPoint, ls.item(0).endSketchPoint,
                                             adsk.fusion.DimensionOrientations.HorizontalDimensionOrientation, P3(2, -1, 0))
d.parameter.expression = "Len"
ext = root.features.extrudeFeatures.addSimple(sk.profiles.item(0), VI("10"), adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
rows = []


def snap(label):
    ps = {p.name: {"expr": p.expression, "unit": p.unit, "value_cm": round(p.value, 6)}
          for p in [design.allParameters.item(i) for i in range(design.allParameters.count)]}
    rows.append({"step": label, "docUnits": um.defaultLengthUnits, "params": ps,
                 "vol_mm3": round(root.bRepBodies.item(0).volume * 1000, 3),
                 "fmt_4cm": um.formatInternalValue(4, um.defaultLengthUnits, True)})


snap("initial (mm); extrude typed '10' unitless")
um.distanceDisplayUnits = adsk.fusion.DistanceUnits.InchDistanceUnits
snap("after document units -> inch")
up.add("NewNum", VI("2"), "in", "")
snap("after adding unitless '2' while doc in inch")
um.distanceDisplayUnits = adsk.fusion.DistanceUnits.MillimeterDistanceUnits
snap("back to mm")
result["rows"] = rows
