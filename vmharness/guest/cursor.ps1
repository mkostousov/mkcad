# Write the guest cursor position (interactive session) to C:\mkprobe\out\cursor.txt
Add-Type -AssemblyName System.Windows.Forms
$p = [System.Windows.Forms.Cursor]::Position
"$($p.X),$($p.Y)" | Set-Content C:\mkprobe\out\cursor.txt
