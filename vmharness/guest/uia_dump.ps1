# Dump the UI Automation tree of the Fusion main window (must run in the interactive session).
param([int]$MaxDepth = 12, [string]$Out = 'C:\mkprobe\out\uia_dump.txt')
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$A = [System.Windows.Automation.AutomationElement]
$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
$root = $null
$top = $walker.GetFirstChild($A::RootElement)
while ($top) {
    if ($top.Current.Name -like '*Autodesk Fusion*') { $root = $top; break }
    $top = $walker.GetNextSibling($top)
}
if (-not $root) { 'Fusion window not found' | Set-Content $Out; exit 2 }
$sb = [System.Text.StringBuilder]::new()
$stats = @{}
function Walk($el, $depth) {
    if ($depth -gt $MaxDepth) { return }
    $c = $el.Current
    $type = $c.ControlType.ProgrammaticName -replace 'ControlType\.', ''
    $stats[$type] = 1 + [int]$stats[$type]
    $r = $c.BoundingRectangle
    $rect = if ($r.IsEmpty) { '' } else { '[{0},{1} {2}x{3}]' -f [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height }
    [void]$sb.AppendLine(('{0}{1} "{2}" id={3} cls={4} {5}' -f ('  ' * $depth), $type, $c.Name, $c.AutomationId, $c.ClassName, $rect))
    $ch = $walker.GetFirstChild($el)
    while ($ch) { Walk $ch ($depth + 1); $ch = $walker.GetNextSibling($ch) }
}
$t = Get-Date
Walk $root 0
"# elements by type: " + (($stats.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ' ') | Set-Content $Out -Encoding utf8
"# seconds: " + [int]((Get-Date) - $t).TotalSeconds | Add-Content $Out -Encoding utf8
$sb.ToString() | Add-Content $Out -Encoding utf8
