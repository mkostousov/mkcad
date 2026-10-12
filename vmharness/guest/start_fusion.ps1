# Start Fusion in the interactive session via its desktop shortcut (run through gui.ps1).
$lnk = Get-ChildItem "$env:USERPROFILE\Desktop", "$env:PUBLIC\Desktop" -Filter '*Fusion*.lnk' -EA SilentlyContinue | Select-Object -First 1
if (-not $lnk) { exit 2 }
Start-Process $lnk.FullName
