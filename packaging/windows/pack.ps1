#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = (Resolve-Path (Join-Path $ScriptDir "..\..")).Path
Set-Location $Root

$SpecPath = Join-Path $Root "packaging\package-spec.json"
$Spec = Get-Content -Raw $SpecPath | ConvertFrom-Json
$Platform = "windows-x86_64"
$Row = $Spec.platforms.$Platform
if (-not $Row) { throw "missing platform $Platform in package-spec.json" }

function Get-MinimixVersion {
    if ($env:MINIMIX_VERSION) { return $env:MINIMIX_VERSION }
    if ($env:GITHUB_REF_NAME -and $env:GITHUB_REF_NAME.StartsWith("v")) {
        return $env:GITHUB_REF_NAME.Substring(1)
    }
    $tag = ""
    try { $tag = (git describe --tags --exact-match 2>$null) } catch { }
    if ($tag -and $tag.StartsWith("v")) { return $tag.Substring(1) }
    $sha = (git rev-parse --short HEAD).Trim()
    return "0.0.0-dev+$sha"
}

$Version = Get-MinimixVersion
$StageName = "minimix-$Version-$Platform"
$Stage = Join-Path $Root "dist\stage\$StageName"
$OutName = "minimix-$Version-$Platform.$($Row.archive)"
$Out = Join-Path $Root "dist\$OutName"

cargo build --release -p minimix-daemon -p minimix
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

Copy-Item (Join-Path $Root "target\release\minimix-daemon.exe") $Stage
Copy-Item (Join-Path $Root "target\release\minimix.exe") $Stage
Copy-Item (Join-Path $ScriptDir "install.ps1") (Join-Path $Stage "install.ps1")

$components = ($Row.components -join ",")
$notes = @(
    "platform: $Platform"
    "signing: $($Row.signing)"
    "components: $components"
    ""
    $Row.notes
    ""
    "--- excerpt from platforms/windows/wave-rt/STATUS.md ---"
    ""
)
$notes += Get-Content (Join-Path $Root "platforms\windows\wave-rt\STATUS.md")
$notes -join "`n" | Set-Content -Encoding utf8 (Join-Path $Stage "NOTES.txt")

New-Item -ItemType Directory -Force -Path (Split-Path $Out) | Out-Null
if (Test-Path $Out) { Remove-Item -Force $Out }
Compress-Archive -Path $Stage -DestinationPath $Out

Write-Host "packed $Out"
