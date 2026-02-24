# NotarAI installer for Windows
#
# Usage:
#   irm https://raw.githubusercontent.com/davidroeca/NotarAI/main/scripts/install.ps1 | iex
#
# Environment variables:
#   $env:VERSION      — release tag to install (default: latest)
#   $env:INSTALL_DIR  — installation directory (default: $env:LOCALAPPDATA\Programs\notarai)

$ErrorActionPreference = "Stop"

$Repo = "davidroeca/NotarAI"
$Binary = "notarai-x86_64-windows.exe"

if (-not $env:INSTALL_DIR) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\notarai"
} else {
    $InstallDir = $env:INSTALL_DIR
}

# Determine version
if (-not $env:VERSION) {
    $Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $Release.tag_name
} else {
    $Version = $env:VERSION
}

if (-not $Version) {
    Write-Error "Could not determine latest version"
    exit 1
}

$Url = "https://github.com/$Repo/releases/download/$Version/$Binary"

Write-Host "Installing notarai $Version to $InstallDir..."

# Create install directory
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$DestPath = Join-Path $InstallDir "notarai.exe"

# Download
Invoke-WebRequest -Uri $Url -OutFile $DestPath

# Add to PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to user PATH (restart your shell to use)"
}

Write-Host "notarai installed to $DestPath"
& $DestPath --version
