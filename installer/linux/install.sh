#!/bin/sh
# Installs OpenECraft from this directory for the current user (~/.local) or, with
# PREFIX=/usr/local and sudo, for all users. `./install.sh --uninstall` removes it.
#
#   ./install.sh
#   sudo PREFIX=/usr/local ./install.sh

set -eu
here="$(cd "$(dirname "$0")" && pwd)"
prefix="${PREFIX:-$HOME/.local}"
files="$(cd "$here" && find bin share -type f)"

if [ "${1:-}" = --uninstall ]; then
    for f in $files; do
        rm -f "$prefix/$f"
    done
    rmdir "$prefix/share/doc/openecraft" 2>/dev/null || true
else
    for f in $files; do
        mkdir -p "$prefix/$(dirname "$f")"
        cp "$here/$f" "$prefix/$f"
    done
fi

if command -v update-desktop-database >/dev/null; then update-desktop-database -q "$prefix/share/applications" || true; fi
if command -v update-mime-database >/dev/null; then update-mime-database "$prefix/share/mime" || true; fi
if command -v gtk-update-icon-cache >/dev/null; then gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" || true; fi

case ":$PATH:" in
    *":$prefix/bin:"*) ;;
    *) [ "${1:-}" = --uninstall ] || echo "Add $prefix/bin to the PATH to start openecraft from a terminal." ;;
esac
