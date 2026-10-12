# X-18 driver: for each case build the scene via API, open Extrude in the GUI, OCR the Operation field, cancel.
param([string[]]$Cases = @('xy_inside', 'xy_partial', 'xy_touch', 'xy_far', 'plane_above', 'top_face_outside', 'two_bodies_top'))
$root = Split-Path $PSScriptRoot -Parent
$py = Join-Path $root '.venv\Scripts\python.exe'
$vmh = Join-Path $root 'vmh.py'
$tpl = Get-Content (Join-Path $root 'fusion\jobs\x18_case.py') -Raw
foreach ($c in $Cases) {
    $job = Join-Path $root "fusion\jobs\x18_$c.py"
    $tpl.Replace('__CASE__', $c) | Set-Content $job -Encoding utf8
    & (Join-Path $root 'fjob.ps1') $job | Out-Null
    $r = (Get-Content (Join-Path $root "results\x18_$c.json") -Raw | ConvertFrom-Json)
    if (-not $r.ok) { "$c : JOB ERROR $($r.error)"; continue }
    & $py $vmh click 194 96 | Out-Null
    Start-Sleep 3
    $ocr = & $py $vmh ocr --crop 1660 280 1900 600
    $op = ($ocr | Where-Object { $_ -match 'Body|Join|Cut|Intersect' } | ForEach-Object { ($_ -split "`t")[1] }) -join ' / '
    & $py $vmh shot -o (Join-Path $root "shots\x18_$c.png") --crop 1660 280 1900 600 --scale 0.6 | Out-Null
    "{0,-18} profiles={1} sel={2}mm2 bodies={3} -> operation: {4}" -f $c, $r.result.profiles, $r.result.sel_area_mm2, $r.result.bodies, $op
    & $py $vmh click 1867 582 | Out-Null          # dialog Cancel button (Esc proved unreliable)
    Start-Sleep 1.5
    & (Join-Path $root 'fjob.ps1') (Join-Path $root 'fusion\jobs\cmd.py') | Out-Null
    $cmd = (Get-Content (Join-Path $root 'results\cmd.json') -Raw | ConvertFrom-Json).result.cmd
    if ($cmd -ne 'SelectCommand') { "  warning: active command still $cmd"; & $py $vmh key esc | Out-Null; Start-Sleep 1 }
}
