# RISE format registration for Windows
# Registers .rise as a system-recognized image/video type.
# Run as Administrator.

$ErrorActionPreference = "Stop"

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Please run as Administrator" -ForegroundColor Red
    exit 1
}

# 1. Register .rise extension with PerceivedType image
New-Item -Path "HKLM:\SOFTWARE\Classes\.rise" -Force | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\.rise" -Name "PerceivedType" -Value "image"
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\.rise" -Name "ContentType" -Value "image/jpeg"
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\.rise" -Name "(default)" -Value "RISE.SealedFile"

# 2. Register the file type description
New-Item -Path "HKLM:\SOFTWARE\Classes\RISE.SealedFile" -Force | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\RISE.SealedFile" -Name "(default)" -Value "RISE Sealed Media"

# 3. Open with any image viewer (shell open verb)
New-Item -Path "HKLM:\SOFTWARE\Classes\RISE.SealedFile\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\RISE.SealedFile\shell\open\command" -Name "(default)" -Value "`"%ProgramFiles%\Windows Photo Viewer\PhotoViewer.dll`", ImageView_Fullscreen %1"

# 4. Register in the Explorer "Save as" known extensions list
New-Item -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\KindMap" -Force -ErrorAction SilentlyContinue | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\KindMap" -Name ".rise" -Value "picture" -ErrorAction SilentlyContinue

Write-Host "RISE format registered on Windows." -ForegroundColor Green
Write-Host ".rise files are now recognized as images by Explorer."
