#!/usr/bin/env bash
# Puts the license files next to the web page: run by trunk after the build of the
# editor and of the parts manager (post_build hook of their Trunk.toml, on Windows
# licenses.ps1). The release build also gets THIRD-PARTY-LICENSES.html and needs
# cargo-about for it (cargo install cargo-about@0.9.3 --locked --features cli).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
dir="${TRUNK_STAGING_DIR:?run by trunk}"

cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$dir/"
cp "$root/crates/openecraft-fonts/fonts/OFL.txt" "$dir/OFL-NotoSans.txt"
if [ "${TRUNK_PROFILE:-}" = release ]; then
    command -v cargo-about >/dev/null \
        || { echo "error: cargo-about not found (cargo install cargo-about@0.9.3 --locked --features cli)" >&2; exit 1; }
    cargo about generate --fail --manifest-path "$root/Cargo.toml" -c "$root/installer/about.toml" \
        -o "$dir/THIRD-PARTY-LICENSES.html" "$root/installer/about.hbs"
fi
