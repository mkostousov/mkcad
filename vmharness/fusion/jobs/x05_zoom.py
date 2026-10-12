# MKProbe job: X-05 — zoom the active view out x2 (viewExtents), report screen position of an empty model spot.
P3 = adsk.core.Point3D.create
vp = app.activeViewport
cam = vp.camera
before = cam.viewExtents
cam.isSmoothTransition = False
cam.viewExtents = before * 2
vp.camera = cam
vp.refresh()


def scr(x, y):
    p = vp.modelToViewSpace(P3(x / 10, y / 10, 0))
    return [round(p.x), round(p.y) + 140]


result["extents"] = [before, vp.camera.viewExtents]
result["start_-60_40"] = scr(-60, 40)
result["px_per_mm"] = round(scr(10, 0)[0] - scr(0, 0)[0], 3) / 10
result["active"] = app.activeEditObject.objectType.split("::")[-1]
