#!/bin/bash
# Builds the macOS disk image target/installer/OpenECraft-<version>-macos.dmg with the
# apps OpenECraft (editor; the command line / MCP server and the parts and project
# servers in OpenECraft.app/Contents/MacOS) and OpenECraft Parts Manager, as universal
# binaries for Apple silicon and Intel:
#
#   bash installer/macos/build.sh
#
# Needs the Xcode command line tools, both Rust targets
# (rustup target add aarch64-apple-darwin x86_64-apple-darwin) and cargo-about for the
# third-party licenses (cargo install cargo-about --locked).
#
# MACOS_TARGETS="aarch64-apple-darwin" builds for one architecture only.
# MACOS_SIGN_IDENTITY="Developer ID Application: …" signs with that certificate
# (hardened runtime), otherwise the apps are signed ad hoc.
# MACOS_NOTARY_PROFILE=<keychain profile of notarytool> notarizes and staples the image
# (MACOS_NOTARY_KEYCHAIN=<keychain> when the profile is not in the login keychain).

set -euo pipefail
umask 022

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="$root/target/installer"
work="$out/macos"
id=io.github.nilres.OpenECraft
version="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
[ -n "$version" ] || { echo "error: no version in [workspace.package] of Cargo.toml" >&2; exit 1; }
command -v cargo-about >/dev/null || { echo "error: cargo-about not found (cargo install cargo-about --locked)" >&2; exit 1; }
# shellcheck disable=SC2206
targets=(${MACOS_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin})
identity="${MACOS_SIGN_IDENTITY:--}"

export MACOSX_DEPLOYMENT_TARGET=11.0
for target in "${targets[@]}"; do
    cargo build --release --target "$target" --manifest-path "$root/Cargo.toml" \
        -p openecraft -p openecraft-parts-manager -p openecraft-cli -p openecraft-parts-server -p openecraft-project-server
done

# Universal binary of one program (the slices of all targets).
universal() {
    local target
    local -a slices
    slices=()
    for target in "${targets[@]}"; do
        slices+=("$root/target/$target/release/$1")
    done
    lipo -create -output "$2" "${slices[@]}"
}

# info_plist <name> <bundle id> <executable>
info_plist() {
    cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleLocalizations</key>
    <array><string>en</string><string>de</string><string>fr</string><string>es</string></array>
    <key>CFBundleName</key><string>$1</string>
    <key>CFBundleDisplayName</key><string>$1</string>
    <key>CFBundleIdentifier</key><string>$2</string>
    <key>CFBundleExecutable</key><string>$3</string>
    <key>CFBundleIconFile</key><string>OpenECraft</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
    <key>LSMinimumSystemVersion</key><string>$MACOSX_DEPLOYMENT_TARGET</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
EOF
}

# The name of the app in the Finder per language: <name> <de> <fr> <es>.
localize() {
    local app="$1" lang name
    shift
    for lang in en de fr es; do
        name="$1"
        shift
        install -d "$app/Contents/Resources/$lang.lproj"
        printf 'CFBundleName = "%s";\nCFBundleDisplayName = "%s";\n' "$name" "$name" > "$app/Contents/Resources/$lang.lproj/InfoPlist.strings"
    done
}

sign() {
    if [ "$identity" = - ]; then
        codesign --force --sign - "$1"
    else
        codesign --force --sign "$identity" --options runtime --timestamp "$1"
    fi
}

# bundle <app> <name> <bundle id> <program>... (the first program is started)
bundle() {
    local app="$1" name="$2" bundle_id="$3" program
    shift 3
    rm -rf "$app"
    install -d "$app/Contents/MacOS" "$app/Contents/Resources"
    info_plist "$name" "$bundle_id" "$1" > "$app/Contents/Info.plist"
    printf 'APPL????' > "$app/Contents/PkgInfo"
    cp "$root/assets/icon.icns" "$app/Contents/Resources/OpenECraft.icns"
    cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$work/THIRD-PARTY-LICENSES.html" "$app/Contents/Resources/"
    cp "$root/crates/openecraft-fonts/fonts/OFL.txt" "$app/Contents/Resources/OFL-NotoSans.txt"
    # The main program is signed with the bundle.
    for program in "$@"; do
        universal "$program" "$app/Contents/MacOS/$program"
        [ "$program" = "$1" ] || sign "$app/Contents/MacOS/$program"
    done
}

dmg_root="$work/dmg"
rm -rf "$work"
install -d "$dmg_root" "$out"
cargo about generate --fail --manifest-path "$root/Cargo.toml" -c "$root/installer/about.toml" \
    -o "$work/THIRD-PARTY-LICENSES.html" "$root/installer/about.hbs"

bundle "$dmg_root/OpenECraft.app" OpenECraft "$id" \
    openecraft openecraft-cli openecraft-parts-server openecraft-project-server
localize "$dmg_root/OpenECraft.app" OpenECraft OpenECraft OpenECraft OpenECraft
sign "$dmg_root/OpenECraft.app"

bundle "$dmg_root/OpenECraft Parts Manager.app" "OpenECraft Parts Manager" "$id.PartsManager" openecraft-parts-manager
localize "$dmg_root/OpenECraft Parts Manager.app" "OpenECraft Parts Manager" \
    "OpenECraft-Artikelverwaltung" "Gestionnaire d'articles OpenECraft" "Gestor de artículos de OpenECraft"
sign "$dmg_root/OpenECraft Parts Manager.app"

ln -s /Applications "$dmg_root/Applications"

dmg="$out/OpenECraft-$version-macos.dmg"
hdiutil create -volname "OpenECraft $version" -srcfolder "$dmg_root" -fs HFS+ -format UDZO -ov "$dmg"
if [ "$identity" != - ]; then
    codesign --force --sign "$identity" --timestamp "$dmg"
fi
if [ -n "${MACOS_NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$dmg" --keychain-profile "$MACOS_NOTARY_PROFILE" \
        ${MACOS_NOTARY_KEYCHAIN:+--keychain "$MACOS_NOTARY_KEYCHAIN"} --wait
    xcrun stapler staple "$dmg"
fi
ls -l "$dmg"
