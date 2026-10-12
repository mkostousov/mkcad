# Open each command by its command-definition id (API execute), OCR its task dialog, save text + image, cancel.
# Needs an open design with a body. Usage: .\tools\dialogs.ps1 -Commands 'Revolve','FusionFilletEdgesCommand'
param([string[]]$Commands = @('Revolve', 'FusionFilletEdgesCommand', 'FusionChamferCommand', 'FusionShellBodyCommand',
                              'FusionDraftCommand', 'FusionHoleCommand', 'PatternRectangular', 'PatternCircular',
                              'MirrorCommand', 'Sweep', 'SolidLoft', 'FusionCombineCommand', 'MeasureCommand'))
$root = Split-Path $PSScriptRoot -Parent
$py = Join-Path $root '.venv\Scripts\python.exe'
$vmh = Join-Path $root 'vmh.py'
$out = Join-Path $root 'inventory\dialogs'
New-Item -ItemType Directory -Force $out | Out-Null
function Cmd { & (Join-Path $root 'fjob.ps1') (Join-Path $root 'fusion\jobs\cmd.py') | Out-Null
               (Get-Content (Join-Path $root 'results\cmd.json') -Raw | ConvertFrom-Json).result.cmd }
$tpl = Get-Content (Join-Path $root 'fusion\jobs\open_cmd.py') -Raw
foreach ($c in $Commands) {
    $job = Join-Path $root 'fusion\jobs\open_cmd_run.py'
    $tpl.Replace('__CMD__', $c) | Set-Content $job -Encoding utf8
    & (Join-Path $root 'fjob.ps1') $job | Out-Null
    Start-Sleep 2.5
    $active = Cmd
    $lines = & $py $vmh ocr --crop 1640 140 1915 990 --bw
    $text = $lines | ForEach-Object { ($_ -split "`t")[1] }
    $text | Set-Content (Join-Path $out "$c.txt")
    & $py $vmh shot -o (Join-Path $out "$c.png") --crop 1640 140 1915 990 --scale 0.6 | Out-Null
    "== {0} [{1}]: {2}" -f $c, $active, ($text -join ' | ')
    $cancel = $lines | Where-Object { $_ -match "`tCancel$" } | Select-Object -Last 1
    if ($cancel) { $x, $y = (($cancel -split "`t")[0] -split ','); & $py $vmh click $x $y | Out-Null } else { & $py $vmh key esc | Out-Null }
    Start-Sleep 1.2
    if ((Cmd) -ne 'SelectCommand') { & $py $vmh key esc | Out-Null; Start-Sleep 1; "  (extra Esc needed)" }
}
