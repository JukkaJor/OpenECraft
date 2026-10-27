#!/usr/bin/env bash
# Builds the Linux packages of OpenECraft into target/installer/:
#
#   bash installer/linux/build.sh [tar.gz] [deb] [rpm] [appimage] [flatpak] [snap]
#
# Without arguments it builds tar.gz, deb, rpm and appimage. The packages contain the
# editor, the parts manager, the command line / MCP server and the parts and project
# servers for the architecture of the build machine. Each format needs its tool:
#
#   deb       dpkg-deb, dpkg-shlibdeps (apt install dpkg-dev)
#   rpm       rpmbuild (dnf install rpm-build, apt install rpm)
#   appimage  appimagetool (APPIMAGETOOL=<path>, otherwise release 1.9.1 is downloaded)
#   flatpak   flatpak, flatpak-builder (the runtime is installed from Flathub)
#   snap      snapcraft (SNAPCRAFT="sudo snapcraft" SNAPCRAFT_ARGS=--destructive-mode in CI)
#
# All formats need cargo-about for the third-party licenses (cargo install cargo-about --locked).
# OPENECRAFT_BIN_DIR=<dir> packages binaries built before instead of running cargo.

set -euo pipefail
umask 022

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="$root/target/installer"
work="$out/linux"
id=io.github.nilres.OpenECraft
bins=(openecraft openecraft-parts-manager openecraft-cli openecraft-parts-server openecraft-project-server)
version="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)"
notices="$out/THIRD-PARTY-LICENSES.html"
# deb and rpm sort `~` before a release ("0.2.0~rc1" < "0.2.0") and do not allow `-`.
pkgver="${version//-/\~}"
arch="$(uname -m)"
formats=("$@")
[ ${#formats[@]} -gt 0 ] || formats=(tar.gz deb rpm appimage)

die() { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "$1 not found, needed for $2"; }

[ -n "$version" ] || die "no version in [workspace.package] of Cargo.toml"
need cargo-about "the third-party licenses (cargo install cargo-about --locked)"
grep -q "<release version=\"$version\"" "$here/$id.metainfo.xml" \
    || die "add <release version=\"$version\" date=\"…\"/> to installer/linux/$id.metainfo.xml"

if [ -n "${OPENECRAFT_BIN_DIR:-}" ]; then
    bin_dir="$OPENECRAFT_BIN_DIR"
else
    cargo build --release --manifest-path "$root/Cargo.toml" \
        -p openecraft -p openecraft-parts-manager -p openecraft-cli -p openecraft-parts-server -p openecraft-project-server
    bin_dir="$root/target/release"
fi

# The installed files below <dir> as for the prefix /usr: bin/, share/.
stage() {
    local dir="$1" b
    rm -rf "$dir"
    install -d "$dir/bin"
    for b in "${bins[@]}"; do
        install -m 755 -s "$bin_dir/$b" "$dir/bin/$b"
    done
    install -D -m 644 "$here/$id.desktop" "$dir/share/applications/$id.desktop"
    install -D -m 644 "$here/$id.PartsManager.desktop" "$dir/share/applications/$id.PartsManager.desktop"
    install -D -m 644 "$here/$id.metainfo.xml" "$dir/share/metainfo/$id.metainfo.xml"
    install -D -m 644 "$here/$id.xml" "$dir/share/mime/packages/$id.xml"
    install -D -m 644 "$root/assets/icon.png" "$dir/share/icons/hicolor/256x256/apps/$id.png"
    install -D -m 644 "$root/assets/icon.svg" "$dir/share/icons/hicolor/scalable/apps/$id.svg"
    install -D -m 644 "$root/README.md" "$dir/share/doc/openecraft/README.md"
    install -D -m 644 "$root/LICENSE-MIT" "$dir/share/doc/openecraft/LICENSE-MIT"
    install -D -m 644 "$root/LICENSE-APACHE" "$dir/share/doc/openecraft/LICENSE-APACHE"
    install -D -m 644 "$root/crates/openecraft-fonts/fonts/OFL.txt" "$dir/share/doc/openecraft/OFL-NotoSans.txt"
    install -D -m 644 "$notices" "$dir/share/doc/openecraft/THIRD-PARTY-LICENSES.html"
}

build_tar() {
    local name="openecraft-$version-linux-$arch"
    stage "$work/$name"
    install -m 755 "$here/install.sh" "$work/$name/install.sh"
    tar -C "$work" --owner=0 --group=0 -czf "$out/OpenECraft-$version-linux-$arch.tar.gz" "$name"
}

build_deb() {
    need dpkg-deb deb
    need dpkg-shlibdeps deb
    local pkg="$work/deb" darch shlibs
    darch="$(dpkg --print-architecture)"
    stage "$pkg/usr"
    cat > "$pkg/usr/share/doc/openecraft/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: OpenECraft
Source: https://github.com/nilres/OpenECraft

Files: *
Copyright: OpenECraft contributors
License: MIT or Apache-2.0
EOF
    # dpkg-shlibdeps wants a debian/control in the current directory.
    mkdir -p "$work/shlibs/debian"
    printf 'Source: openecraft\n\nPackage: openecraft\n' > "$work/shlibs/debian/control"
    shlibs="$(cd "$work/shlibs" && dpkg-shlibdeps -O "$pkg"/usr/bin/* | sed -n 's/^shlibs:Depends=//p')"
    install -d "$pkg/DEBIAN"
    # The window system and OpenGL libraries are loaded at run time, dpkg-shlibdeps
    # does not see them.
    cat > "$pkg/DEBIAN/control" <<EOF
Package: openecraft
Version: $pkgver
Architecture: $darch
Maintainer: ${MAINTAINER:-OpenECraft <https://github.com/nilres/OpenECraft/issues>}
Installed-Size: $(du -sk "$pkg/usr" | cut -f 1)
Depends: $shlibs, libxkbcommon0, libgl1, libegl1
Recommends: libwayland-client0, libwayland-egl1, libx11-6, libxcursor1, libxi6, libxrandr2, libxkbcommon-x11-0, xdg-desktop-portal
Section: electronics
Priority: optional
Homepage: https://github.com/nilres/OpenECraft
Description: Electrical CAE editor: schematics, panel layouts and reports
 OpenECraft is an open electrical CAE editor in the spirit of EPLAN Electric P8:
 schematic pages with device tags, cross references and a netlist, panel layout
 in 2D and 3D, checks, reports and exports.
 .
 The package contains the editor, the parts manager, the command line and MCP
 server for AI agents and the parts and project servers.
EOF
    dpkg-deb --root-owner-group --build "$pkg" "$out/openecraft_${pkgver}_$darch.deb"
}

build_rpm() {
    need rpmbuild rpm
    stage "$work/rpm/files"
    rpmbuild -bb --target "$arch" \
        --define "_topdir $work/rpm" \
        --define "pkg_version $pkgver" \
        --define "files_dir $work/rpm/files" \
        "$here/openecraft.spec"
    cp "$work/rpm/RPMS/$arch/openecraft-$pkgver-1.$arch.rpm" "$out/"
}

build_appimage() {
    local dir="$work/OpenECraft.AppDir" tool="${APPIMAGETOOL:-}"
    stage "$dir/usr"
    install -m 755 "$here/appimage/AppRun" "$dir/AppRun"
    cp "$here/$id.desktop" "$dir/$id.desktop"
    cp "$root/assets/icon.png" "$dir/$id.png"
    ln -sf "$id.png" "$dir/.DirIcon"
    if [ -z "$tool" ]; then
        tool="$(command -v appimagetool || true)"
    fi
    if [ -z "$tool" ]; then
        tool="$work/appimagetool-1.9.1-$arch.AppImage"
        [ -x "$tool" ] || {
            need curl appimage
            curl -fL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$arch.AppImage"
            chmod +x "$tool"
        }
    fi
    # Extract-and-run works without FUSE (containers, CI).
    ARCH="$arch" APPIMAGE_EXTRACT_AND_RUN=1 "$tool" "$dir" "$out/OpenECraft-$version-$arch.AppImage"
}

build_flatpak() {
    need flatpak flatpak
    need flatpak-builder flatpak
    local dir="$work/flatpak"
    install -d "$dir"
    stage "$dir/files"
    cp "$here/flatpak/$id.yml" "$dir/$id.yml"
    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak-builder --user --force-clean --disable-rofiles-fuse --install-deps-from=flathub \
        --state-dir="$dir/.flatpak-builder" --repo="$dir/repo" "$dir/build" "$dir/$id.yml"
    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
        "$dir/repo" "$out/OpenECraft-$version-$arch.flatpak" "$id"
}

build_snap() {
    local dir="$work/snap" snapcraft="${SNAPCRAFT:-snapcraft}"
    need "${snapcraft##* }" snap
    rm -rf "$dir"
    stage "$dir/files/usr"
    # snapd does not look up theme icons of the snap, the entries name the file.
    sed -i "s|^Icon=.*|Icon=\${SNAP}/usr/share/icons/hicolor/256x256/apps/$id.png|" "$dir/files/usr/share/applications/"*.desktop
    install -d "$dir/snap/gui"
    cp "$root/assets/icon.png" "$dir/snap/gui/openecraft.png"
    sed "s/@VERSION@/$version/" "$here/snap/snapcraft.yaml" > "$dir/snap/snapcraft.yaml"
    # SNAPCRAFT and SNAPCRAFT_ARGS are split into words on purpose.
    # shellcheck disable=SC2086
    (cd "$dir" && $snapcraft pack ${SNAPCRAFT_ARGS:-})
    mv "$dir"/openecraft_*.snap "$out/"
}

install -d "$work"
cargo about generate --fail --manifest-path "$root/Cargo.toml" -c "$root/installer/about.toml" \
    -o "$notices" "$root/installer/about.hbs"
for format in "${formats[@]}"; do
    case "$format" in
        tar.gz) build_tar ;;
        deb) build_deb ;;
        rpm) build_rpm ;;
        appimage) build_appimage ;;
        flatpak) build_flatpak ;;
        snap) build_snap ;;
        *) die "unknown format $format (tar.gz, deb, rpm, appimage, flatpak, snap)" ;;
    esac
done
ls -l "$out"
