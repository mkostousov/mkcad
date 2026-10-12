# Resume watcher for the main chat: run as a background task; its exit wakes the chat.
# Exits when local time passes -Until, or when the signal file appears (deleted on exit).
# Usage: .\tools\watcher.ps1 -Until '2026-10-11T11:35:00-04:00'
param([Parameter(Mandatory)][datetime]$Until,
      [string]$Signal = 'D:\Users\mikhail\SynologyDrive\Work\AI\mk_cad\.resume')
while ((Get-Date) -lt $Until -and -not (Test-Path $Signal)) { Start-Sleep -Seconds 30 }
$why = if (Test-Path $Signal) { Remove-Item $Signal -Force; 'signal file' } else { 'time reached' }
"RESUME fusion study ($why) at $(Get-Date -Format s)"
