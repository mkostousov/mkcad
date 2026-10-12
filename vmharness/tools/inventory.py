"""Condense results/dump_ui.json into compact inventory files.

Outputs (inventory/):
  workspaces.md         workspace -> tab -> panel -> control tree (names + command ids)
  commands.csv          command id, name, tooltip, controlType, placements (workspace/tab/panel)
Usage: python tools/inventory.py [workspace_id ...]   (default: all workspaces with tabs)
"""
import csv
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
dump = json.loads((ROOT / "results" / "dump_ui.json").read_text(encoding="utf-8"))["result"]
out = ROOT / "inventory"
out.mkdir(exist_ok=True)

wanted = set(sys.argv[1:])
placements = defaultdict(list)
md = [f"# Fusion UI inventory (version {dump['app']['version']})", "",
      "Source: Fusion API dump (`dump_ui.py`). `*` = promoted to panel face, `(hidden)` = isVisible false.", ""]


def walk(ctrls, depth, where):
    for c in ctrls:
        cmd = c.get("command") or {}
        label = cmd.get("name") or c.get("name") or c.get("id")
        if c.get("type") == "SeparatorControl":
            continue
        flags = ("*" if c.get("isPromoted") is True else "") + ("" if c.get("isVisible") is not False else " (hidden)")
        cid = cmd.get("id") or c.get("id")
        md.append(f"{'  ' * depth}- {label}{flags} `{cid}`")
        if cmd.get("id"):
            placements[cmd["id"]].append(where)
        if c.get("children"):
            walk(c["children"], depth + 1, where)


for ws in dump["workspaces"]:
    if not ws["tabs"] or (wanted and ws["id"] not in wanted):
        continue
    md += [f"## {ws['name']} `{ws['id']}`", ""]
    for tab in ws["tabs"]:
        md.append(f"### {tab['name']} `{tab['id']}`" + ("" if tab.get("isVisible") is not False else " (hidden)"))
        for p in tab["panels"]:
            md.append(f"- **{p['name']}** `{p['id']}`" + ("" if p.get("isVisible") is not False else " (hidden)"))
            walk(p["controls"], 1, f"{ws['id']}/{tab['id']}/{p['id']}")
        md.append("")

(out / "workspaces.md").write_text("\n".join(md), encoding="utf-8")

with open(out / "commands.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["id", "name", "tooltip", "controlType", "isNative", "placements"])
    for cd in dump["commandDefinitions"]:
        tip = (cd.get("tooltip") or "").replace("\n", " ").strip()
        w.writerow([cd["id"], cd.get("name"), tip, cd.get("controlType"), cd.get("isNative"),
                    ";".join(sorted(set(placements.get(cd["id"], []))))])

print(f"workspaces.md {len(md)} lines; commands.csv {len(dump['commandDefinitions'])} rows; "
      f"{len(placements)} commands placed in toolbars")
