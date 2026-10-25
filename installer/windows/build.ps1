# Builds the Windows setup: the release binaries with a static C runtime (no Visual C++
# redistributable needed on the target machine) and the Inno Setup package
# target\installer\OpenECraft-<version>-setup.exe.
#
#   powershell -ExecutionPolicy Bypass -File installer\windows\build.ps1
#
# Needs Inno Setup 6 (winget install JRSoftware.InnoSetup).

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$target = 'x86_64-pc-windows-msvc'

$version = (Select-String -Path "$root\Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --target $target --manifest-path "$root\Cargo.toml" `
    -p openecraft -p openecraft-parts-manager -p openecraft-cli -p openecraft-parts-server -p openecraft-project-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$iscc = @(
    (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source,
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 not found: winget install JRSoftware.InnoSetup' }

& $iscc "/DAppVersion=$version" "/DBinDir=$root\target\$target\release" "/DOutputDir=$root\target\installer" "$PSScriptRoot\openecraft.iss"
exit $LASTEXITCODE
