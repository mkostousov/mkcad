"""VM harness: screen capture + mouse/keyboard control of a VirtualBox guest via the COM API.

Usage (run with .venv\\Scripts\\python.exe):
  vmh.py shot [-o file] [--crop x0 y0 x1 y1] [--scale f]
  vmh.py res W H                         set guest resolution hint
  vmh.py move X Y
  vmh.py click X Y [--button left|right|middle] [--double] [--hold ctrl]
  vmh.py drag X1 Y1 X2 Y2 [--button ...] [--steps N] [--hold shift]
  vmh.py scroll X Y N                    N>0 scrolls down, N<0 up
  vmh.py type "text"                     US-layout text entry
  vmh.py key ctrl+s [enter ...]          key chords, applied in order
  vmh.py wait SECONDS
  vmh.py session start NAME | stop | note "text" | report
Coordinates are guest pixels, origin top-left. While a session is active,
every action is logged to steps.jsonl with an annotated "after" screenshot.
"""
from __future__ import annotations

import argparse
import io
import json
import sys
import time
from datetime import datetime
from pathlib import Path

import win32com.client
from PIL import Image, ImageDraw

VM_NAME = "Win10x64 AI Sandbox"
ROOT = Path(__file__).resolve().parent
STATE_FILE = ROOT / ".state.json"
SESSIONS_DIR = ROOT / "sessions"
SHOTS_DIR = ROOT / "shots"

LOCK_SHARED = 1
BITMAP_PNG = 0x20474E50
BUTTONS = {"left": 1, "right": 2, "middle": 4}
SETTLE = 0.6  # seconds to let the UI repaint before an "after" screenshot


# ---------------------------------------------------------------- VM access
class Vm:
    def __init__(self, name: str = VM_NAME):
        self._client = win32com.client.Dispatch("VirtualBox.VirtualBoxClient")
        self._machine = self._client.VirtualBox.FindMachine(name)
        self._session = self._client.Session

    def __enter__(self) -> "Vm":
        self._machine.LockMachine(self._session, LOCK_SHARED)
        con = self._session.Console
        self.display, self.mouse, self.keyboard = con.Display, con.Mouse, con.Keyboard
        return self

    def __exit__(self, *exc) -> None:
        self._session.UnlockMachine()

    def resolution(self) -> tuple[int, int]:
        w, h, *_ = self.display.GetScreenResolution(0)
        return w, h

    def screenshot(self) -> Image.Image:
        w, h = self.resolution()
        data = self.display.TakeScreenShotToArray(0, w, h, BITMAP_PNG)
        return Image.open(io.BytesIO(bytes(data))).convert("RGB")

    # VirtualBox absolute coordinates are 1-based.
    def mouse_to(self, x: int, y: int, buttons: int = 0, dz: int = 0) -> None:
        self.mouse.PutMouseEventAbsolute(x + 1, y + 1, dz, 0, buttons)

    def scancodes(self, codes: list[int]) -> None:
        for c in codes:
            self.keyboard.PutScancode(c)
            time.sleep(0.008)


# ---------------------------------------------------------------- keyboard map (scancode set 1, US layout)
_ROWS = {
    0x02: "1!", 0x03: "2@", 0x04: "3#", 0x05: "4$", 0x06: "5%", 0x07: "6^", 0x08: "7&",
    0x09: "8*", 0x0A: "9(", 0x0B: "0)", 0x0C: "-_", 0x0D: "=+", 0x1A: "[{", 0x1B: "]}",
    0x27: ";:", 0x28: "'\"", 0x29: "`~", 0x2B: "\\|", 0x33: ",<", 0x34: ".>", 0x35: "/?",
}
_LETTERS = "qwertyuiop", 0x10, "asdfghjkl", 0x1E, "zxcvbnm", 0x2C
CHAR_MAP: dict[str, tuple[int, bool]] = {}
for code, pair in _ROWS.items():
    CHAR_MAP[pair[0]] = (code, False)
    CHAR_MAP[pair[1]] = (code, True)
for row, start in zip(_LETTERS[0::2], _LETTERS[1::2]):
    for i, ch in enumerate(row):
        CHAR_MAP[ch] = (start + i, False)
        CHAR_MAP[ch.upper()] = (start + i, True)
CHAR_MAP.update({" ": (0x39, False), "\n": (0x1C, False), "\t": (0x0F, False)})

# name -> (scancode, extended)
KEYS: dict[str, tuple[int, bool]] = {
    "esc": (0x01, False), "escape": (0x01, False), "backspace": (0x0E, False), "tab": (0x0F, False),
    "enter": (0x1C, False), "return": (0x1C, False), "space": (0x39, False), "capslock": (0x3A, False),
    "ctrl": (0x1D, False), "shift": (0x2A, False), "alt": (0x38, False),
    "rctrl": (0x1D, True), "rshift": (0x36, False), "ralt": (0x38, True),
    "win": (0x5B, True), "lwin": (0x5B, True), "menu": (0x5D, True),
    "up": (0x48, True), "down": (0x50, True), "left": (0x4B, True), "right": (0x4D, True),
    "home": (0x47, True), "end": (0x4F, True), "pageup": (0x49, True), "pgup": (0x49, True),
    "pagedown": (0x51, True), "pgdn": (0x51, True), "insert": (0x52, True), "delete": (0x53, True),
    "del": (0x53, True),
    **{f"f{i}": (0x3A + i, False) for i in range(1, 11)}, "f11": (0x57, False), "f12": (0x58, False),
}
for ch, (code, shifted) in CHAR_MAP.items():
    if not shifted and ch.strip():
        KEYS.setdefault(ch, (code, False))


def _press(code: int, ext: bool) -> list[int]:
    return [0xE0, code] if ext else [code]


def _release(code: int, ext: bool) -> list[int]:
    return [0xE0, code | 0x80] if ext else [code | 0x80]


def chord_codes(chord: str) -> list[int]:
    names = [n.strip().lower() for n in chord.split("+")]
    keys = []
    for n in names:
        if n not in KEYS:
            raise SystemExit(f"unknown key: {n}")
        keys.append(KEYS[n])
    down = [c for k in keys for c in _press(*k)]
    up = [c for k in reversed(keys) for c in _release(*k)]
    return down + up


def text_codes(text: str) -> list[int]:
    out: list[int] = []
    shift = KEYS["shift"]
    for ch in text:
        if ch not in CHAR_MAP:
            raise SystemExit(f"cannot type character {ch!r} (US layout map)")
        code, shifted = CHAR_MAP[ch]
        if shifted:
            out += _press(*shift)
        out += [code, code | 0x80]
        if shifted:
            out += _release(*shift)
    return out


# ---------------------------------------------------------------- session log
def load_state() -> dict:
    try:
        return json.loads(STATE_FILE.read_text())
    except (OSError, ValueError):
        return {}


def save_state(state: dict) -> None:
    STATE_FILE.write_text(json.dumps(state, indent=2))


def annotate(img: Image.Image, marks: list[tuple[int, int]], path: list[tuple[int, int]] | None = None) -> Image.Image:
    img = img.copy()
    d = ImageDraw.Draw(img)
    if path and len(path) > 1:
        d.line(path, fill=(255, 0, 255), width=3)
    for x, y in marks:
        for r, col in ((18, (255, 0, 0)), (4, (255, 0, 0))):
            d.ellipse([x - r, y - r, x + r, y + r], outline=col, width=3)
        d.line([x - 28, y, x - 20, y], fill=(255, 0, 0), width=3)
        d.line([x + 20, y, x + 28, y], fill=(255, 0, 0), width=3)
        d.line([x, y - 28, x, y - 20], fill=(255, 0, 0), width=3)
        d.line([x, y + 20, x, y + 28], fill=(255, 0, 0), width=3)
    return img


def log_step(vm: Vm, action: str, args: dict, marks=(), path=None, note: str = "") -> Path | None:
    state = load_state()
    sess = state.get("session")
    if not sess:
        return None
    sdir = Path(sess)
    n = state.get("step", 0) + 1
    time.sleep(SETTLE)
    shot = annotate(vm.screenshot(), list(marks), path)
    name = f"{n:04d}_{action}.png"
    shot.save(sdir / name)
    entry = {"n": n, "time": datetime.now().isoformat(timespec="seconds"), "action": action,
             "args": args, "note": note, "screenshot": name}
    with open(sdir / "steps.jsonl", "a", encoding="utf-8") as f:
        f.write(json.dumps(entry) + "\n")
    state["step"] = n
    save_state(state)
    return sdir / name


# ---------------------------------------------------------------- commands
def cmd_shot(a) -> None:
    with Vm() as vm:
        img = vm.screenshot()
    if a.crop:
        img = img.crop(tuple(a.crop))
    if a.scale != 1.0:
        img = img.resize((max(1, int(img.width * a.scale)), max(1, int(img.height * a.scale))), Image.LANCZOS)
    out = Path(a.out) if a.out else SHOTS_DIR / f"shot_{datetime.now():%Y%m%d_%H%M%S}.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    print(f"{out} {img.width}x{img.height}")


def ocr_image(img: Image.Image, upscale: float = 2.0) -> list[dict]:
    """Windows built-in OCR. Returns lines with boxes in the coordinates of `img`."""
    import asyncio
    from winrt.windows.graphics.imaging import BitmapDecoder
    from winrt.windows.media.ocr import OcrEngine
    from winrt.windows.storage.streams import DataWriter, InMemoryRandomAccessStream

    big = img.resize((int(img.width * upscale), int(img.height * upscale)), Image.LANCZOS)
    buf = io.BytesIO()
    big.save(buf, "PNG")

    async def run():
        stream = InMemoryRandomAccessStream()
        w = DataWriter(stream)
        w.write_bytes(buf.getvalue())
        await w.store_async()
        stream.seek(0)
        bmp = await (await BitmapDecoder.create_async(stream)).get_software_bitmap_async()
        return await OcrEngine.try_create_from_user_profile_languages().recognize_async(bmp)

    res = asyncio.run(run())
    lines = []
    for ln in res.lines:
        rs = [wd.bounding_rect for wd in ln.words]
        x0, y0 = min(r.x for r in rs), min(r.y for r in rs)
        x1, y1 = max(r.x + r.width for r in rs), max(r.y + r.height for r in rs)
        lines.append({"text": ln.text, "box": [round(v / upscale) for v in (x0, y0, x1, y1)]})
    return lines


def cmd_ocr(a) -> None:
    if a.file:
        img = Image.open(a.file).convert("RGB")
    else:
        with Vm() as vm:
            img = vm.screenshot()
    ox, oy = 0, 0
    if a.crop:
        ox, oy = a.crop[0], a.crop[1]
        img = img.crop(tuple(a.crop))
    if a.bw:  # caller knows the region sits on the canvas grid: threshold first
        img = img.convert("L").point(lambda p: 255 if p > 150 else 0).convert("RGB")
    lines = ocr_image(img)
    if not lines:  # canvas grid lines defeat the engine: drop light pixels (grid) and keep dark text
        lines = ocr_image(img.convert("L").point(lambda p: 255 if p > 150 else 0).convert("RGB"))
    for ln in lines:
        x0, y0, x1, y1 = ln["box"]
        if a.find and a.find.lower() not in ln["text"].lower():
            continue
        # center point in screen coordinates: directly usable for `click`
        print(f"{(x0 + x1) // 2 + ox},{(y0 + y1) // 2 + oy}\t{ln['text']}")


def cmd_res(a) -> None:
    with Vm() as vm:
        vm.display.SetVideoModeHint(0, True, False, 0, 0, a.w, a.h, 32, True)
        for _ in range(20):
            time.sleep(0.5)
            if vm.resolution() == (a.w, a.h):
                break
        print("resolution", vm.resolution())


def cmd_move(a) -> None:
    with Vm() as vm:
        vm.mouse_to(a.x, a.y)
        _report(log_step(vm, "move", vars_of(a), [(a.x, a.y)], note=a.note))


def hold_codes(hold: str | None, down: bool) -> list[int]:
    if not hold:
        return []
    keys = [KEYS[n.strip().lower()] for n in hold.split("+")]
    if down:
        return [c for k in keys for c in _press(*k)]
    return [c for k in reversed(keys) for c in _release(*k)]


def cmd_click(a) -> None:
    b = BUTTONS[a.button]
    with Vm() as vm:
        vm.scancodes(hold_codes(a.hold, True))
        vm.mouse_to(a.x, a.y)
        time.sleep(0.05)
        for _ in range(2 if a.double else 1):
            vm.mouse_to(a.x, a.y, b)
            time.sleep(0.05)
            vm.mouse_to(a.x, a.y, 0)
            time.sleep(0.08)
        vm.scancodes(hold_codes(a.hold, False))
        _report(log_step(vm, ("dbl" if a.double else "") + f"click_{a.button}", vars_of(a), [(a.x, a.y)], note=a.note))


def cmd_drag(a) -> None:
    b = BUTTONS[a.button]
    pts = [(round(a.x1 + (a.x2 - a.x1) * i / a.steps), round(a.y1 + (a.y2 - a.y1) * i / a.steps))
           for i in range(a.steps + 1)]
    with Vm() as vm:
        vm.scancodes(hold_codes(a.hold, True))
        vm.mouse_to(a.x1, a.y1)
        time.sleep(0.05)
        vm.mouse_to(a.x1, a.y1, b)
        time.sleep(0.1)
        for x, y in pts[1:]:
            vm.mouse_to(x, y, b)
            time.sleep(0.015)
        time.sleep(0.1)
        vm.mouse_to(a.x2, a.y2, 0)
        vm.scancodes(hold_codes(a.hold, False))
        _report(log_step(vm, f"drag_{a.button}", vars_of(a), [(a.x1, a.y1), (a.x2, a.y2)], pts, note=a.note))


def cmd_scroll(a) -> None:
    with Vm() as vm:
        vm.mouse_to(a.x, a.y)
        step = 1 if a.n > 0 else -1
        for _ in range(abs(a.n)):
            vm.mouse_to(a.x, a.y, 0, step)
            time.sleep(0.05)
        _report(log_step(vm, "scroll", vars_of(a), [(a.x, a.y)], note=a.note))


def cmd_type(a) -> None:
    with Vm() as vm:
        vm.scancodes(text_codes(a.text))
        _report(log_step(vm, "type", vars_of(a), note=a.note))


def cmd_key(a) -> None:
    with Vm() as vm:
        for chord in a.chords:
            vm.scancodes(chord_codes(chord))
            time.sleep(0.05)
        _report(log_step(vm, "key", vars_of(a), note=a.note))


def cmd_keydown(a) -> None:
    with Vm() as vm:
        vm.scancodes(hold_codes(a.keys, True))


def cmd_keyup(a) -> None:
    with Vm() as vm:
        vm.scancodes(hold_codes(a.keys, False))


def cmd_wait(a) -> None:
    time.sleep(a.seconds)


def cmd_session(a) -> None:
    state = load_state()
    if a.op == "start":
        if not a.arg:
            raise SystemExit("session start NAME")
        sdir = SESSIONS_DIR / f"{datetime.now():%Y%m%d_%H%M%S}_{a.arg}"
        sdir.mkdir(parents=True)
        save_state({"session": str(sdir), "step": 0})
        with Vm() as vm:
            vm.screenshot().save(sdir / "0000_start.png")
        print(sdir)
    elif a.op == "stop":
        save_state({})
        print("stopped", state.get("session"))
    elif a.op == "note":
        if not state.get("session"):
            raise SystemExit("no active session")
        with open(Path(state["session"]) / "steps.jsonl", "a", encoding="utf-8") as f:
            f.write(json.dumps({"n": state.get("step", 0), "action": "note", "note": a.arg}) + "\n")
    elif a.op == "report":
        sdir = Path(a.arg) if a.arg else Path(state.get("session", ""))
        print(write_report(sdir))


def write_report(sdir: Path) -> Path:
    lines = [f"# Session {sdir.name}", "", "![start](0000_start.png)", ""]
    for raw in (sdir / "steps.jsonl").read_text(encoding="utf-8").splitlines():
        e = json.loads(raw)
        if e["action"] == "note":
            lines += [f"> {e['note']}", ""]
            continue
        args = ", ".join(f"{k}={v}" for k, v in e["args"].items() if k not in ("note",) and v not in (None, False))
        lines += [f"## Step {e['n']}: {e['action']}", "", f"`{args}`" + (f" — {e['note']}" if e["note"] else ""),
                  "", f"![step {e['n']}]({e['screenshot']})", ""]
    out = sdir / "report.md"
    out.write_text("\n".join(lines), encoding="utf-8")
    return out


def vars_of(a) -> dict:
    return {k: v for k, v in vars(a).items() if k not in ("func",)}


def _report(p: Path | None) -> None:
    print(f"logged {p}" if p else "ok")


def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(prog="vmh")
    sub = p.add_subparsers(required=True)

    s = sub.add_parser("shot"); s.set_defaults(func=cmd_shot)
    s.add_argument("-o", "--out"); s.add_argument("--crop", nargs=4, type=int); s.add_argument("--scale", type=float, default=1.0)

    s = sub.add_parser("ocr"); s.set_defaults(func=cmd_ocr)
    s.add_argument("--file"); s.add_argument("--crop", nargs=4, type=int); s.add_argument("--find")
    s.add_argument("--bw", action="store_true", help="threshold dark text before OCR")

    s = sub.add_parser("res"); s.set_defaults(func=cmd_res)
    s.add_argument("w", type=int); s.add_argument("h", type=int)

    def note(sp): sp.add_argument("--note", default="")
    def hold(sp): sp.add_argument("--hold", help="modifiers held during the action, e.g. shift or ctrl+shift")

    s = sub.add_parser("move"); s.set_defaults(func=cmd_move); s.add_argument("x", type=int); s.add_argument("y", type=int); note(s)
    s = sub.add_parser("click"); s.set_defaults(func=cmd_click); s.add_argument("x", type=int); s.add_argument("y", type=int)
    s.add_argument("--button", choices=BUTTONS, default="left"); s.add_argument("--double", action="store_true"); note(s); hold(s)
    s = sub.add_parser("drag"); s.set_defaults(func=cmd_drag)
    for n in ("x1", "y1", "x2", "y2"):
        s.add_argument(n, type=int)
    s.add_argument("--button", choices=BUTTONS, default="left"); s.add_argument("--steps", type=int, default=20); note(s); hold(s)
    s = sub.add_parser("scroll"); s.set_defaults(func=cmd_scroll); s.add_argument("x", type=int); s.add_argument("y", type=int)
    s.add_argument("n", type=int); note(s)
    s = sub.add_parser("type"); s.set_defaults(func=cmd_type); s.add_argument("text"); note(s)
    s = sub.add_parser("key"); s.set_defaults(func=cmd_key); s.add_argument("chords", nargs="+"); note(s)
    s = sub.add_parser("keydown"); s.set_defaults(func=cmd_keydown); s.add_argument("keys", help="e.g. ctrl or ctrl+shift")
    s = sub.add_parser("keyup"); s.set_defaults(func=cmd_keyup); s.add_argument("keys")
    s = sub.add_parser("wait"); s.set_defaults(func=cmd_wait); s.add_argument("seconds", type=float)
    s = sub.add_parser("session"); s.set_defaults(func=cmd_session)
    s.add_argument("op", choices=["start", "stop", "note", "report"]); s.add_argument("arg", nargs="?")

    a = p.parse_args(argv)
    a.func(a)


if __name__ == "__main__":
    main()
