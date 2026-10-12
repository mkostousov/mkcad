"""MKProbe: Fusion add-in that runs Python job files dropped into C:\\mkprobe\\in.

Each job runs on Fusion's main thread (via a custom event) with globals:
  adsk, app, ui, result (dict to fill), log(msg)
Output: C:\\mkprobe\\out\\<job>.json  {ok, elapsed, result, log, error, traceback}
"""
import json
import os
import threading
import time
import traceback

import adsk.core
import adsk.fusion

ROOT = r"C:\mkprobe"
IN_DIR, OUT_DIR, DONE_DIR = (os.path.join(ROOT, d) for d in ("in", "out", "done"))
EVENT_ID = "MKProbeJob"

_app = None
_handlers = []
_event = None
_stop = threading.Event()


def _run_job(path):
    name = os.path.splitext(os.path.basename(path))[0]
    logs = []
    env = {"adsk": adsk, "app": _app, "ui": _app.userInterface, "result": {},
           "log": lambda m: logs.append(str(m)), "__name__": "__mkprobe_job__"}
    out = {"job": name, "ok": False}
    t0 = time.perf_counter()
    try:
        with open(path, encoding="utf-8") as f:
            code = compile(f.read(), path, "exec")
        exec(code, env)
        out["ok"] = True
    except Exception as e:  # report everything; never crash Fusion
        out["error"] = repr(e)
        out["traceback"] = traceback.format_exc()
    out["elapsed"] = round(time.perf_counter() - t0, 3)
    out["result"] = env["result"]
    out["log"] = logs
    tmp = os.path.join(OUT_DIR, name + ".json.tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, default=str)
    os.replace(tmp, os.path.join(OUT_DIR, name + ".json"))
    os.replace(path, os.path.join(DONE_DIR, os.path.basename(path)))


class _JobHandler(adsk.core.CustomEventHandler):
    def notify(self, args):
        try:
            _run_job(args.additionalInfo)
        except Exception:
            with open(os.path.join(OUT_DIR, "_addin_error.txt"), "a", encoding="utf-8") as f:
                f.write(traceback.format_exc())


def _poll():
    pending = set()
    while not _stop.wait(0.5):
        try:
            for fn in sorted(os.listdir(IN_DIR)):
                p = os.path.join(IN_DIR, fn)
                if fn.endswith(".py") and p not in pending:
                    pending.add(p)
                    _app.fireCustomEvent(EVENT_ID, p)
            pending = {p for p in pending if os.path.exists(p)}
        except Exception:
            pass


def run(context):
    global _app, _event
    _app = adsk.core.Application.get()
    for d in (IN_DIR, OUT_DIR, DONE_DIR):
        os.makedirs(d, exist_ok=True)
    _event = _app.registerCustomEvent(EVENT_ID)
    h = _JobHandler()
    _event.add(h)
    _handlers.append(h)
    _stop.clear()
    threading.Thread(target=_poll, daemon=True).start()
    with open(os.path.join(OUT_DIR, "_ready.txt"), "w") as f:
        f.write(time.strftime("%Y-%m-%d %H:%M:%S") + " " + _app.version)


def stop(context):
    _stop.set()
    if _event and _handlers:
        _event.remove(_handlers[0])
    _app.unregisterCustomEvent(EVENT_ID)
