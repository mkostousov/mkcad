# Resume guide — clean-room Fusion study

Read first: `docs/STUDY.md` (environment, findings F1–F118, stability log, next steps), spec drafts in `docs/spec/`, and the brief
`D:/Users/mikhail/SynologyDrive/Work/AI/clean-room-cad-fsd-prompt.md`.

## 1. Bring the lab up (all commands from `D:\Users\mikhail\SynologyDrive\Work\AI\mk_cad\vmharness`, PowerShell)

1. VM state: `& "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" showvminfo "Win10x64 AI Sandbox" --machinereadable | Select-String '^VMState='`
   - If `poweroff`: `& "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" startvm "Win10x64 AI Sandbox" --type gui`, wait for `ssh -o BatchMode=yes ai-sandbox "echo up"`.
2. Logged in? `.\rps.ps1 '"explorer=" + @(Get-Process explorer -EA SilentlyContinue).Count'`
   - `explorer=0` → the VM is at the sign-in screen. Autologon is NOT configured and Claude must not type the password.
     Notify the user (push notification) and stop.
3. Resolution: `.\.venv\Scripts\python.exe vmh.py res 1920 1080`
4. Fusion running? `ssh ai-sandbox "type C:\mkprobe\out\_ready.txt"` is rewritten at each Fusion start.
   To start: `ssh ai-sandbox "del /q C:\mkprobe\out\_ready.txt"` then `.\gui.ps1 guest\start_fusion.ps1`, poll `_ready.txt` (≈1 min).
5. Mouse sanity: `.\gui.ps1 guest\mouseinfo.ps1; ssh ai-sandbox "type C:\mkprobe\out\mouseinfo.txt"` — `lbutton=True` means the
   guest button is stuck; ask the user to click once inside the VM window.

## 2. Working rules (credit efficiency)

- Prefer Fusion API jobs (`.\fjob.ps1 fusion\jobs\<job>.py`, results in `results\`) → text only. Summarise results with PowerShell; never dump big JSON.
- GUI: OCR first (`vmh.py ocr --crop ...`), cropped/scaled screenshots only when a picture is the evidence.
- Keyboard: click the target field, wait ~1 s, type, verify via API/OCR (focus can be lost).
- Experiments only in unsaved Untitled documents (Fusion Personal: 10 editable docs limit). `fusion\jobs\close_unsaved.py` closes them.
- Record every finding in `docs/STUDY.md` (ID, classification, evidence path).
- Check plan usage (ccd_session_mgmt get_usage) every ~10 steps. Keep working until ≥95 % of the 5-hour or weekly window;
  then update STUDY.md "Next steps", schedule the resume (below) shortly after the reported reset, and stop.
- Long sessions are fine (user preference, 2026-10-11): do not hand off just because context is large — auto-compaction handles it.
  Keep STUDY.md current after every finding so nothing is lost on compaction.
- Resume structure (user decision 2026-10-11): ALL work happens in the main chat
  (session `local_a6a992e2-df54-41bf-82af-8341c8a28fa5`, "CAD software functionality mapping"). Scheduled sessions never do work.
  At ≥95 % usage: update STUDY.md, then (a) start `vmharness\tools\watcher.ps1 -Until <reset + 5 min>` as a BACKGROUND task in
  the main chat — its exit wakes the chat; (b) re-arm task `fusion-study-resume` (update_scheduled_task fireAt = reset + 5 min,
  enabled) as backup — it only sends "RESUME" to the main chat (or creates `mk_cad\.resume`, which ends the watcher).
- Never edit Claude Code permission/settings files; if a prompt blocks progress, record it in STUDY.md and notify the user.

## 3. Continue with

`docs/STUDY.md` → "Next steps".
