# Run a PowerShell script block in the guest over ssh without quoting issues.
# Usage: .\rps.ps1 'Get-Process | Select -First 3'
param([Parameter(Mandatory)][string]$Script)
$enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes("`$ProgressPreference='SilentlyContinue'`n" + $Script))
ssh ai-sandbox "powershell -NoProfile -NonInteractive -EncodedCommand $enc"
exit $LASTEXITCODE
