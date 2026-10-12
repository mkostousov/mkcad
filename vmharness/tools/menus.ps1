# Open each ribbon panel dropdown of the current tab, save a cropped image and OCR text per menu.
# Usage: .\tools\menus.ps1 -Tab SOLID  (Fusion design must be open; tab must be active)
param([string]$Tab = 'SOLID', [string[]]$Only)
$py = Join-Path $PSScriptRoot '..\.venv\Scripts\python.exe'
$vmh = Join-Path $PSScriptRoot '..\vmh.py'
$out = Join-Path $PSScriptRoot "..\inventory\menus\$Tab"
New-Item -ItemType Directory -Force $out | Out-Null
$panels = & $py $vmh ocr --crop 0 110 1554 140 | ForEach-Object {
    $xy, $name = $_ -split "`t"; $x, $y = $xy -split ','
    [pscustomobject]@{ Name = ($name -replace "[^A-Za-z ]", '').Trim(); X = [int]$x; Y = [int]$y }
}
if ($Only) { $panels = $panels | Where-Object Name -in $Only }
foreach ($p in $panels) {
    $x0 = [Math]::Max(0, $p.X - 160); $x1 = [Math]::Min(1554, $p.X + 340)
    $n = 0
    for ($try = 0; $try -lt 3 -and $n -lt 3; $try++) {
        & $py $vmh click $p.X $p.Y | Out-Null   # toggles: a stale open menu may absorb a click
        Start-Sleep -Milliseconds 1200
        & $py $vmh shot -o "$out\$($p.Name).png" --crop $x0 135 $x1 990 | Out-Null
        $txt = @(& $py $vmh ocr --file "$out\$($p.Name).png" | ForEach-Object { ($_ -split "`t")[1] })
        $n = $txt.Count
    }
    $txt | Set-Content "$out\$($p.Name).txt"
    & $py $vmh click 800 600 | Out-Null          # click empty canvas to close the menu
    Start-Sleep -Milliseconds 400
    "{0}: {1} lines (tries {2})" -f $p.Name, $n, $try
}
