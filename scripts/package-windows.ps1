param(
    [switch]$PortableOnly,
    [string]$MakensisPath
)

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
$PortablePackage = Join-Path $Dist "Jump1-$Version-x86_64-windows.zip"
$InstallerPackage = Join-Path $Dist "Jump1-$Version-x86_64-windows-setup.exe"

if (-not $PortableOnly -and -not $MakensisPath) {
    $MakensisCommand = Get-Command 'makensis.exe' -ErrorAction SilentlyContinue
    if ($MakensisCommand) {
        $MakensisPath = $MakensisCommand.Source
    } else {
        $MakensisCandidates = @(
            (Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'NSIS\makensis.exe'),
            (Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'NSIS\makensis.exe')
        )
        $MakensisPath = $MakensisCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    }
}

if (-not $PortableOnly -and (-not $MakensisPath -or -not (Test-Path -LiteralPath $MakensisPath -PathType Leaf))) {
    throw 'NSIS makensis.exe is required to create the Windows installer. Install NSIS or pass -MakensisPath; use -PortableOnly to build just the ZIP.'
}

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
    Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $PortablePackage -CompressionLevel Optimal -Force
} finally {
    Remove-Item -LiteralPath $Stage -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output "Created: $PortablePackage"

if (-not $PortableOnly) {
    $InstallerScript = Join-Path $Root 'installer\windows.nsi'
    $Icon = Join-Path $Root 'assets\app-icon.ico'
    Push-Location $Root
    try {
        & $MakensisPath "/DAPP_VERSION=$Version" "/DOUTPUT_FILE=$InstallerPackage" "/DAPP_BINARY=$Binary" "/DAPP_ICON=$Icon" $InstallerScript
        if ($LASTEXITCODE -ne 0) {
            throw "NSIS compilation failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }

    if (-not (Test-Path -LiteralPath $InstallerPackage -PathType Leaf)) {
        throw "Windows installer was not created: $InstallerPackage"
    }

    Write-Output "Created: $InstallerPackage"
}
