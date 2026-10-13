$ErrorActionPreference = 'Stop'

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$CargoToml = Get-Content (Join-Path $Root 'Cargo.toml') -Raw
if ($CargoToml -notmatch '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"') {
    throw 'Could not read a stable version from Cargo.toml.'
}
$Version = $Matches.version
$Binary = Join-Path $Root 'target\release\jump1.exe'
$Dist = Join-Path $Root 'dist'
$Stage = Join-Path $Root ("target\package-windows-" + [guid]::NewGuid().ToString('N'))
$Package = Join-Path $Dist "Jump1-$Version-x86_64-windows.zip"

Push-Location $Root
try {
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Windows release binary was not created: $Binary"
}

New-Item -ItemType Directory -Path $Dist -Force | Out-Null
New-Item -ItemType Directory -Path $Stage | Out-Null
try {
    Copy-Item -LiteralPath $Binary -Destination (Join-Path $Stage 'Jump1.exe')
    @'
Jump1 for Windows

Extract this ZIP to a folder you can write to, then run Jump1.exe.
To update, use Settings > General > Software Updates inside the app.
'@ | Set-Content -LiteralPath (Join-Path $Stage 'README-Windows.txt') -Encoding UTF8
    Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $Package -CompressionLevel Optimal -Force
} finally {
    Remove-Item -LiteralPath $Stage -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "Created: $Package"
