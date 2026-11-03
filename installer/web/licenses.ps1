# Puts the license files next to the web page: run by trunk after the build of the
# editor and of the parts manager on Windows (post_build hook of their Trunk.toml,
# elsewhere licenses.sh). The release build also gets THIRD-PARTY-LICENSES.html and
# needs cargo-about for it (cargo install cargo-about@0.9.3 --locked --features cli).

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..\..").Path
$dir = $env:TRUNK_STAGING_DIR
if (-not $dir) { throw 'run by trunk' }

Copy-Item "$root\LICENSE-MIT", "$root\LICENSE-APACHE" $dir
Copy-Item "$root\crates\openecraft-fonts\fonts\OFL.txt" "$dir\OFL-NotoSans.txt"
if ($env:TRUNK_PROFILE -eq 'release') {
    if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
        throw 'cargo-about not found: cargo install cargo-about@0.9.3 --locked --features cli'
    }
    cargo about generate --fail --manifest-path "$root\Cargo.toml" -c "$root\installer\about.toml" `
        -o "$dir\THIRD-PARTY-LICENSES.html" "$root\installer\about.hbs"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
