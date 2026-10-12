# Log left-button down/up transitions for 8 s in the interactive session.
Add-Type -Name U -Namespace W -MemberDefinition '[DllImport("user32.dll")] public static extern short GetAsyncKeyState(int k);'
$log = @("start@" + [DateTime]::Now.ToString('ss.fff') + " initial=" + [bool]([W.U]::GetAsyncKeyState(1) -band 0x8000))
$prev = $null
$end = (Get-Date).AddSeconds(8)
while ((Get-Date) -lt $end) {
    $down = [bool]([W.U]::GetAsyncKeyState(1) -band 0x8000)
    if ($down -ne $prev) { $log += ("{0}@{1}" -f ($(if ($down) { 'DOWN' } else { 'UP' })), [DateTime]::Now.ToString('ss.fff')); $prev = $down }
    Start-Sleep -Milliseconds 5
}
$log -join ' ' | Set-Content C:\mkprobe\out\buttons.txt
