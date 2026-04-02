#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the pathfinder CLI for Satisfactory factory planning.

.DESCRIPTION
    Downloads the latest release from GitHub, extracts the binary, installs it
    to %LOCALAPPDATA%\Programs\pathfinder\, and adds it to the user PATH.

.PARAMETER InstallDir
    Override the default install directory.

.EXAMPLE
    irm https://raw.githubusercontent.com/korti11/satisfactory-pathfinder/master/install/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -InstallDir "C:\tools\pathfinder"
#>
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\pathfinder")
)

$ErrorActionPreference = 'Stop'

$Repo = "korti11/satisfactory-pathfinder"
$ArchiveName = "pathfinder-windows-x86_64.zip"
$BinaryName = "pathfinder.exe"

function Write-Step([string]$Message) {
    Write-Host "  $Message" -ForegroundColor Cyan
}

function Write-Success([string]$Message) {
    Write-Host "  $Message" -ForegroundColor Green
}

Write-Host ""
Write-Host "pathfinder installer" -ForegroundColor White
Write-Host "====================" -ForegroundColor White

# Fetch latest release version
Write-Step "Fetching latest release..."
$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Version = $Release.tag_name
Write-Step "Latest version: $Version"

# Download archive
$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$ArchiveName"
$TmpDir = Join-Path $env:TEMP "pathfinder-install-$([System.IO.Path]::GetRandomFileName())"
$ArchivePath = Join-Path $TmpDir $ArchiveName

Write-Step "Downloading $ArchiveName..."
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath

# Extract
Write-Step "Extracting..."
Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

# Install binary
Write-Step "Installing to $InstallDir..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $TmpDir $BinaryName) (Join-Path $InstallDir $BinaryName) -Force

# Add to user PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$InstallDir", "User")
    Write-Step "Added $InstallDir to user PATH."
}

# Clean up
Remove-Item $TmpDir -Recurse -Force

Write-Host ""
Write-Success "pathfinder $Version installed successfully."
Write-Host ""
Write-Host "  Restart your terminal to pick up the PATH change, then run:" -ForegroundColor Gray
Write-Host "    pathfinder --version" -ForegroundColor White
Write-Host ""
Write-Host "  To install the companion agent for Claude Code:" -ForegroundColor Gray
Write-Host "    pathfinder companion install --global" -ForegroundColor White
Write-Host ""
