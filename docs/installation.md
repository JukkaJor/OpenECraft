# Installation and building

Packages for Windows, Linux and macOS, the web version, and building from source.

Building from source requires a recent stable Rust toolchain (`rustup update stable`).

## Desktop (Windows / Linux / macOS)

```bash
cargo run --release
```

On Linux you need the usual windowing / dialog libraries, e.g. on Debian/Ubuntu:

```bash
sudo apt install libxkbcommon-dev libgtk-3-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

A project file given on the command line is opened at start (`openecraft plant.oec`).

## Windows installer

`OpenECraft-<version>-setup.exe` installs the editor with a Start menu entry, without
administrator rights (or for all users in Program Files), in English, German, French or
Spanish after the Windows language. Optional: the parts manager, the command line / MCP
server (added to the `PATH` on request), the parts and project servers, a desktop
shortcut, and opening `.oec` / `.oecz` files by double-click. It is uninstalled in
*Settings › Apps*; projects and settings are kept. CI builds it on every push (artifact
`openecraft-windows-installer`); locally it needs
[Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`):

```bash
powershell -ExecutionPolicy Bypass -File installer/windows/build.ps1   # → target/installer/
```

The binaries link the C runtime statically, so no Visual C++ redistributable is needed.
For unattended installation (software distribution) run the setup with
`/VERYSILENT /SUPPRESSMSGBOXES`, add `/ALLUSERS` for all users and select parts with
`/COMPONENTS="main,partsmanager,cli,servers"` and `/TASKS="fileassoc,addtopath"`. The
installer is not code signed yet, so SmartScreen asks for confirmation on first run.
`installer/icon.py` draws the application icon (`assets/icon.png`, `assets/icon.ico`,
`assets/icon.icns`, `assets/icon.svg`).

## Linux packages

`installer/linux/build.sh` builds the packages for the architecture of the build machine
into `target/installer/`. Each contains the editor and the parts manager (with menu
entries in English, German, French and Spanish), the command line / MCP server and the
parts and project servers. With deb, rpm, Flatpak and tar.gz `.oec` / `.oecz` files open
by double-click.

```bash
bash installer/linux/build.sh                        # tar.gz, deb, rpm, AppImage
bash installer/linux/build.sh flatpak snap           # sandboxed packages
```

| Package | File | Install | Build tool |
| --- | --- | --- | --- |
| Debian, Ubuntu | `openecraft_<version>_amd64.deb` | `sudo apt install ./openecraft_*.deb` | `dpkg-dev` |
| Fedora, RHEL, openSUSE | `openecraft-<version>-1.x86_64.rpm` | `sudo dnf install ./openecraft-*.rpm` | `rpmbuild` |
| Any distribution | `OpenECraft-<version>-x86_64.AppImage` | `chmod +x`, start it | `appimagetool` (downloaded) |
| Any distribution | `OpenECraft-<version>-x86_64.flatpak` | `flatpak install --user OpenECraft-*.flatpak` | `flatpak-builder` |
| Any distribution | `openecraft_<version>_amd64.snap` | `sudo snap install --dangerous openecraft_*.snap` | `snapcraft` |
| Any distribution | `OpenECraft-<version>-linux-x86_64.tar.gz` | `./install.sh` (to `~/.local`) | `tar` |

The programs are `openecraft`, `openecraft-parts-manager`, `openecraft-cli`,
`openecraft-parts-server` and `openecraft-project-server`. In the AppImage they are
started with `OpenECraft-*.AppImage parts-manager|cli|parts-server|project-server …`, in
the Flatpak with `flatpak run --command=openecraft-cli io.github.nilres.OpenECraft …`
and in the snap as `openecraft.cli`, `openecraft.parts-manager`, … CI builds them on every
push (artifacts `openecraft-linux-packages`, `openecraft-flatpak`, `openecraft-snap`);
deb, rpm, AppImage and tar.gz are built on Debian 12 and run with glibc 2.36 or newer.
The desktop entries, MIME types and AppStream data (`io.github.nilres.OpenECraft`) are
in `installer/linux/`; a new version needs a `<release>` in the metainfo file.

## macOS disk image

`installer/macos/build.sh` builds `OpenECraft-<version>-macos.dmg` (macOS 11 or newer,
universal binaries for Apple silicon and Intel) with the apps *OpenECraft* and
*OpenECraft Parts Manager*; drag them to *Applications*. The command line and the servers
are in the app bundle:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
bash installer/macos/build.sh                        # → target/installer/
/Applications/OpenECraft.app/Contents/MacOS/openecraft-cli help
```

Without `MACOS_SIGN_IDENTITY` the apps are signed ad hoc, so Gatekeeper asks for
confirmation on first start (*System Settings › Privacy & Security › Open Anyway*). With
a Developer ID certificate (`MACOS_SIGN_IDENTITY="Developer ID Application: …"`) and a
`notarytool` keychain profile (`MACOS_NOTARY_PROFILE`) the image is signed and notarized.
Projects are opened with *File › Open project*; opening them by double-click in the Finder is not
supported yet. CI builds the image on every push (artifact `openecraft-macos-dmg`).

## Web (WebAssembly)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve          # development server on http://127.0.0.1:8080
trunk build --release  # static files in dist/ – host them on any web server
```

In the browser "Save" downloads the project file and "Open" uses the file picker.

## Tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo clippy -p openecraft -p openecraft-parts-manager --target wasm32-unknown-unknown
```

All three must pass without warnings (CI runs them with `-D warnings`).
