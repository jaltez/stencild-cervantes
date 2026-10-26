# stencild installer (generated, Windows). Run in PowerShell:
#   .\install.ps1 -Prefix C:\Users\you\AppData\Local\stencild
param([string]$Prefix = "$env:LOCALAPPDATA\stencild")
New-Item -ItemType Directory -Force "$Prefix\bin" | Out-Null
Copy-Item "$PSScriptRoot\stencild.exe" "$Prefix\bin\stencild.exe" -Force
Copy-Item "$PSScriptRoot\tray-icon.png" "$Prefix\bin\tray-icon.png" -Force
Write-Host "installed: $Prefix\bin\stencild.exe"
