$ErrorActionPreference = "Stop"

$Repo    = "Milanzor/spidergrep"
$Bin     = "spidergrep"
$Target  = "x86_64-pc-windows-msvc"

# Fetch latest release tag
Write-Host "Fetching latest release..."
$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Tag     = $Release.tag_name

$Archive = "$Bin-$Tag-$Target.zip"
$Url     = "https://github.com/$Repo/releases/download/$Tag/$Archive"
$Tmp     = Join-Path $env:TEMP $Archive

Write-Host "Downloading $Bin $Tag..."
Invoke-WebRequest -Uri $Url -OutFile $Tmp

Write-Host "Extracting..."
Expand-Archive -Path $Tmp -DestinationPath $env:TEMP -Force

# Install to %LOCALAPPDATA%\Programs\spidergrep
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\spidergrep"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Move-Item -Force (Join-Path $env:TEMP "$Bin.exe") (Join-Path $InstallDir "$Bin.exe")
Remove-Item $Tmp

# Add to PATH for current user if not already present
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your PATH (restart your terminal to apply)"
}

Write-Host "Installed $Bin $Tag to $InstallDir\$Bin.exe"
