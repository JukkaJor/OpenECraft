# Releasing

How a version of OpenECraft is released. A tag `v<version>` starts
`.github/workflows/release.yml`: it checks the version, builds every package with the
jobs of `ci.yml` and creates a draft GitHub release with them.

## Steps

1. Set the version in `[workspace.package]` of the root `Cargo.toml` (all crates and
   installers take it from there) and run `cargo check` to update `Cargo.lock`.
2. Add `<release version="<version>" date="<yyyy-mm-dd>"/>` at the top of `<releases>`
   in `installer/linux/io.github.nilres.OpenECraft.metainfo.xml`.
3. Give the version's section in `CHANGELOG.md` its date (`## [<version>] - <yyyy-mm-dd>`)
   and start a new `unreleased` section after the release.
4. Commit, then tag and push:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

5. When the workflow is done, check the draft under *Releases*: the files, their
   `SHA256SUMS` and the notes (the changelog section). Publish it.

The workflow refuses a tag that does not match the version, a version without a
`<release>` in the metainfo file and a version without a section in the changelog.

## Release files

| File | Built by |
| --- | --- |
| `OpenECraft-<version>-setup.exe` | `installer/windows/build.ps1` |
| `openecraft_<version>_amd64.deb`, `openecraft-<version>-1.x86_64.rpm`, `OpenECraft-<version>-x86_64.AppImage`, `OpenECraft-<version>-linux-x86_64.tar.gz` | `installer/linux/build.sh` |
| `OpenECraft-<version>-x86_64.flatpak`, `openecraft_<version>_amd64.snap` | `installer/linux/build.sh flatpak snap` |
| `OpenECraft-<version>-macos.dmg` | `installer/macos/build.sh` |
| `OpenECraft-<version>-web.zip`, `OpenECraft-Parts-Manager-<version>-web.zip` | `trunk build --release` |
| `THIRD-PARTY-LICENSES.html`, `SHA256SUMS` | the workflow |

## Licenses

OpenECraft is licensed under MIT (`LICENSE-MIT`) or Apache-2.0 (`LICENSE-APACHE`). The
installer scripts generate `THIRD-PARTY-LICENSES.html` from the licenses of all
libraries in the programs with [cargo-about](https://github.com/EmbarkStudios/cargo-about)
(`cargo install cargo-about --locked`; configuration `installer/about.toml`, template
`installer/about.hbs`), and ship it with both license files and the license of the
bundled Noto fonts (`OFL-NotoSans.txt`). A new dependency under a license that is not
in `accepted` of `installer/about.toml` stops the installer build. Add the license only
if it allows distribution in closed and open programs alike, like the ones listed.

## Code signing

Without the secrets below the packages are built unsigned: SmartScreen and Gatekeeper
then ask the user for confirmation on first start. Set them as repository secrets
(*Settings › Secrets and variables › Actions*) to sign in CI; pull requests from forks
never see them.

| Secret | Content |
| --- | --- |
| `WINDOWS_CERTIFICATE` | Code signing certificate as a `.pfx` file, base64 (`base64 -w0 cert.pfx`) |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password of the `.pfx` file |
| `MACOS_CERTIFICATE` | *Developer ID Application* certificate with its key as a `.p12` file, base64 |
| `MACOS_CERTIFICATE_PASSWORD` | Password of the `.p12` file |
| `MACOS_SIGN_IDENTITY` | Name of the certificate, `Developer ID Application: <name> (<team id>)` |
| `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | Apple account, team and app-specific password for notarization |

On Windows `build.ps1` signs the programs, the setup and the uninstaller with
`signtool` (`WINDOWS_SIGN_CERT=<.pfx file>`, `WINDOWS_SIGN_PASSWORD`). Certificate
authorities issue new code signing certificates only on hardware tokens or in cloud
signing services, which cannot be exported as a `.pfx`; such a certificate needs a
signing step of its provider in place of the `.pfx` (for Azure Trusted Signing the
`azure/trusted-signing-action`). On macOS `build.sh` signs the apps with the hardened
runtime and notarizes and staples the disk image (see [installation](installation.md)).
