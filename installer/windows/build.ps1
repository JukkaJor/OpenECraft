# Builds the Windows setup: the release binaries with a static C runtime (no Visual C++
# redistributable needed on the target machine) and the Inno Setup package
# target\installer\OpenECraft-<version>-setup.exe.
#
#   powershell -ExecutionPolicy Bypass -File installer\windows\build.ps1
#
# Needs Inno Setup 6 (winget install JRSoftware.InnoSetup) and cargo-about for the
# third-party licenses (cargo install cargo-about@0.9.3 --locked --features cli).
#
# WINDOWS_SIGN_CERT=<.pfx file> and WINDOWS_SIGN_PASSWORD sign the programs and the setup
# with signtool (Windows SDK); otherwise they stay unsigned.

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$target = 'x86_64-pc-windows-msvc'
$work = "$root\target\installer\windows"

$manifest = Get-Content "$root\Cargo.toml" -Raw
if ($manifest -notmatch '(?ms)^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"') {
    throw 'no version in [workspace.package] of Cargo.toml'
}
$version = $Matches[1]
if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
    throw 'cargo-about not found: cargo install cargo-about@0.9.3 --locked --features cli'
}

$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --locked --target $target --manifest-path "$root\Cargo.toml" `
    -p openecraft -p openecraft-parts-manager -p openecraft-cli -p openecraft-parts-server -p openecraft-project-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$iscc = @(
    (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source,
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 not found: winget install JRSoftware.InnoSetup' }

New-Item -ItemType Directory -Force $work | Out-Null
cargo about generate --fail --manifest-path "$root\Cargo.toml" -c "$root\installer\about.toml" `
    -o "$work\THIRD-PARTY-LICENSES.html" "$root\installer\about.hbs"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$sign = @()
if ($env:WINDOWS_SIGN_CERT) {
    $signtool = @(
        (Get-Command signtool.exe -ErrorAction SilentlyContinue).Source,
        (Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending | Select-Object -First 1).FullName
    ) | Where-Object { $_ } | Select-Object -First 1
    if (-not $signtool) { throw 'signtool not found (Windows SDK)' }
    $signArgs = 'sign', '/fd', 'sha256', '/tr', 'http://timestamp.digicert.com', '/td', 'sha256',
        '/f', $env:WINDOWS_SIGN_CERT, '/p', $env:WINDOWS_SIGN_PASSWORD
    foreach ($exe in 'openecraft', 'openecraft-parts-manager', 'openecraft-cli', 'openecraft-parts-server', 'openecraft-project-server') {
        & $signtool @signArgs "$root\target\$target\release\$exe.exe"
        if ($LASTEXITCODE -ne 0) { throw "signing $exe.exe failed" }
    }
    # Inno Setup signs the setup and the uninstaller ($q is a quote, $f the file).
    $sign = '/DSignTool', ('/Ssigntool=$q' + $signtool + '$q ' + (($signArgs | ForEach-Object { if ($_ -match '\s') { '$q' + $_ + '$q' } else { $_ } }) -join ' ') + ' $f')
}

& $iscc "/DAppVersion=$version" "/DBinDir=$root\target\$target\release" "/DOutputDir=$root\target\installer" `
    "/DNotices=$work\THIRD-PARTY-LICENSES.html" @sign "$PSScriptRoot\openecraft.iss"
exit $LASTEXITCODE
