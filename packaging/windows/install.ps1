#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$BinDir = if ($env:MINIMIX_BIN_DIR) { $env:MINIMIX_BIN_DIR } else {
    Join-Path $env:LOCALAPPDATA "Minimix\bin"
}
$StartMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$MakeShortcut = $env:MINIMIX_START_MENU
if (-not $MakeShortcut) { $MakeShortcut = "ask" }

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
Copy-Item -Force (Join-Path $Here "minimix-daemon.exe") (Join-Path $BinDir "minimix-daemon.exe")
Copy-Item -Force (Join-Path $Here "minimix.exe") (Join-Path $BinDir "minimix.exe")

$pathParts = [Environment]::GetEnvironmentVariable("Path", "User") -split ";" | Where-Object { $_ }
if ($pathParts -notcontains $BinDir) {
    $newPath = ($pathParts + $BinDir) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$BinDir;$env:Path"
    Write-Host "Added $BinDir to the user PATH (new shells pick this up)."
}

function Install-Shortcut {
    New-Item -ItemType Directory -Force -Path $StartMenu | Out-Null
    $lnk = Join-Path $StartMenu "Minimix.lnk"
    $w = New-Object -ComObject WScript.Shell
    $s = $w.CreateShortcut($lnk)
    $s.TargetPath = Join-Path $BinDir "minimix.exe"
    $s.WorkingDirectory = $BinDir
    $s.Description = "Minimix"
    $s.Save()
    Write-Host "Start Menu shortcut: $lnk"
}

switch ($MakeShortcut.ToLowerInvariant()) {
    { $_ -in @("1", "yes", "true") } { Install-Shortcut }
    { $_ -in @("0", "no", "false") } { }
    default {
        $ans = Read-Host "Create a Start Menu shortcut for Minimix? [y/N]"
        if ($ans -match '^[yY]') { Install-Shortcut }
    }
}

Write-Host "Installed minimix.exe and minimix-daemon.exe to $BinDir"
Write-Host "WaveRT kernel driver is not part of this archive. See STATUS.md excerpt in NOTES.txt."
