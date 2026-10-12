# Draw one sketch line in the GUI between two screen points (Line tool via L), optional modifier hold on the end click,
# optional cropped screenshot before the end click; then Esc twice and print the new sketch state (curves + constraints).
param([int]$x0, [int]$y0, [int]$x1, [int]$y1, [string]$hold = '', [string]$shot = '', [string]$holdmove = '')
$v = "$PSScriptRoot\..\.venv\Scripts\python.exe"
$vmh = "$PSScriptRoot\..\vmh.py"
& $v $vmh move 700 300 | Out-Null
& $v $vmh key l | Out-Null
Start-Sleep -Milliseconds 800
& $v $vmh click $x0 $y0 | Out-Null
Start-Sleep -Milliseconds 400
if ($holdmove) { & $v $vmh keydown $holdmove | Out-Null }   # held for the whole move and the end click
& $v $vmh move ([int](($x0 + $x1) / 2)) ([int](($y0 + $y1) / 2)) | Out-Null
& $v $vmh move ($x1 - 3) $y1 | Out-Null
& $v $vmh move $x1 $y1 | Out-Null
Start-Sleep -Milliseconds 800
if ($shot) {
    & $v $vmh shot -o $shot --crop ([Math]::Min($x0, $x1) - 60) ([Math]::Min($y0, $y1) - 90) ([Math]::Max($x0, $x1) + 90) ([Math]::Max($y0, $y1) + 50) | Out-Null
}
if ($hold) { & $v $vmh click $x1 $y1 --hold $hold | Out-Null } else { & $v $vmh click $x1 $y1 | Out-Null }
if ($holdmove) { & $v $vmh keyup $holdmove | Out-Null }
Start-Sleep -Milliseconds 400
& $v $vmh key esc | Out-Null
& $v $vmh key esc | Out-Null
Start-Sleep -Milliseconds 400
Push-Location "$PSScriptRoot\.."
.\fjob.ps1 fusion\jobs\infer_state.py | Out-Null
$r = (Get-Content results\infer_state.json -Raw | ConvertFrom-Json).result
$c = $r.curves[-1]
"last={0}: {1}->{2} | cons={3} | n={4} active={5}" -f $c.t, ($c.s -join ','), (($c.e -join ',') + ' sh=' + ($c.sh -join '/')), ($r.cons -join ','), $r.curves.Count, $r.active
Pop-Location
