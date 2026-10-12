# Report live mouse settings from the interactive session.
Add-Type -Name S -Namespace W -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint a, uint b, ref bool c, uint d);
[DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
[DllImport("user32.dll")] public static extern short GetAsyncKeyState(int k);
[DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);
'@
$cl = $false; [void][W.S]::SystemParametersInfo(0x101E, 0, [ref]$cl, 0)   # SPI_GETMOUSECLICKLOCK
$sb = New-Object System.Text.StringBuilder 256; [void][W.S]::GetWindowText([W.S]::GetForegroundWindow(), $sb, 256)
"clicklock=$cl swapped=$([W.S]::GetSystemMetrics(23)) lbutton=$([bool]([W.S]::GetAsyncKeyState(1) -band 0x8000)) rbutton=$([bool]([W.S]::GetAsyncKeyState(2) -band 0x8000)) foreground='$sb'" | Set-Content C:\mkprobe\out\mouseinfo.txt
