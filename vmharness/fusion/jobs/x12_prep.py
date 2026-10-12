# MKProbe job: X-12 selection priority — fresh box 40x20x10 mm, iso view, screen coordinates of probe points.
# Canvas top-left on screen is (0,140) (UIA, F7); view space is canvas-relative.
P3 = adsk.core.Point3D.create
for i in reversed(range(app.documents.count)):
    d = app.documents.item(i)
    if not d.isSaved:
        d.close(False)
app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
design = adsk.fusion.Design.cast(app.activeProduct)
root = design.rootComponent
sk = root.sketches.add(root.xYConstructionPlane)
sk.sketchCurves.sketchLines.addTwoPointRectangle(P3(0, 0, 0), P3(4, 2, 0))
root.features.extrudeFeatures.addSimple(sk.profiles.item(0), adsk.core.ValueInput.createByString("10 mm"),
                                        adsk.fusion.FeatureOperations.NewBodyFeatureOperation)
sk.isVisible = False
vp = app.activeViewport
cam = vp.camera
cam.viewOrientation = adsk.core.ViewOrientations.IsoTopRightViewOrientation
cam.isSmoothTransition = False
vp.camera = cam
vp.fit()
ui.activeSelections.clear()


def scr(x, y, z):
    p = vp.modelToViewSpace(P3(x / 10, y / 10, z / 10))   # mm -> cm
    return [round(p.x), round(p.y) + 140]


corner = scr(40, 20, 10)       # top vertex
edge_mid = scr(20, 20, 10)     # midpoint of top back edge (y=20)
face_c = scr(20, 10, 10)       # top face centre
pts = {"vertex": corner, "edge_mid": edge_mid, "face_center": face_c}
# probes along the top edge away from the vertex (px), and from the edge into the face (px)
import math
ex, ey = edge_mid[0] - corner[0], edge_mid[1] - corner[1]
el = math.hypot(ex, ey)
fx, fy = face_c[0] - edge_mid[0], face_c[1] - edge_mid[1]
fl = math.hypot(fx, fy)
for d in (2, 4, 6, 8, 12):
    pts[f"vertex+{d}px_along_edge"] = [round(corner[0] + ex / el * d), round(corner[1] + ey / el * d)]
    pts[f"edge+{d}px_into_face"] = [round(edge_mid[0] + fx / fl * d), round(edge_mid[1] + fy / fl * d)]
result["points"] = pts
