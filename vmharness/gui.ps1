# Run a guest PowerShell script inside the interactive desktop session (via a one-shot scheduled task)
# and wait for it to finish. Usage: .\gui.ps1 guest\uia_dump.ps1 [-TimeoutSec 180]
param([Parameter(Mandatory)][string]$Script, [int]$TimeoutSec = 180, [switch]$NoWait)
$name = Split-Path $Script -Leaf
scp -q $Script "ai-sandbox:C:/mkprobe/$name"
$tr = "powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File C:\mkprobe\$name"
ssh ai-sandbox "schtasks /create /f /tn MKProbeGui /sc once /st 00:00 /it /rl highest /tr `"$tr`" >nul && schtasks /run /tn MKProbeGui >nul"
if ($NoWait) { return }
$deadline = (Get-Date).AddSeconds($TimeoutSec)
do {
    Start-Sleep 2
    $st = ssh ai-sandbox "schtasks /query /tn MKProbeGui /fo csv /nh"
} while ($st -match '"Running"' -and (Get-Date) -lt $deadline)
"task: $st"
