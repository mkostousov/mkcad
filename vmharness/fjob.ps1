# Run a Fusion job script through the MKProbe add-in and fetch its JSON result.
# Usage: .\fjob.ps1 fusion\jobs\dump_ui.py [-TimeoutSec 120]
# Result is saved to results\<job>.json; a short status line is printed.
param([Parameter(Mandatory)][string]$Job, [int]$TimeoutSec = 120)
$ErrorActionPreference = 'Stop'
$name = [IO.Path]::GetFileNameWithoutExtension($Job)
$resDir = Join-Path $PSScriptRoot 'results'
New-Item -ItemType Directory -Force $resDir | Out-Null
ssh ai-sandbox "del /q C:\mkprobe\out\$name.json 2>nul" | Out-Null
Remove-Item "$resDir\$name.json" -EA SilentlyContinue   # else a recent stale result passes the freshness check
scp -q $Job "ai-sandbox:C:/mkprobe/$name.py.part"
ssh ai-sandbox "move /y C:\mkprobe\$name.py.part C:\mkprobe\in\$name.py >nul"
$deadline = (Get-Date).AddSeconds($TimeoutSec)
do {
    Start-Sleep -Milliseconds 700
    scp -q "ai-sandbox:C:/mkprobe/out/$name.json" "$resDir\$name.json" 2>$null
} until ((Test-Path "$resDir\$name.json" -NewerThan (Get-Date).AddSeconds(-$TimeoutSec - 5)) -or (Get-Date) -gt $deadline)
if (-not (Test-Path "$resDir\$name.json")) { Write-Error "timeout waiting for $name" }
$r = Get-Content "$resDir\$name.json" -Raw | ConvertFrom-Json
"{0} ok={1} elapsed={2}s size={3}KB {4}" -f $name, $r.ok, $r.elapsed, [int]((Get-Item "$resDir\$name.json").Length / 1KB), $r.error
if (-not $r.ok) { $r.traceback }
