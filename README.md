# OpenECraft

An open source electrical CAE editor written in Rust, in the spirit of EPLAN Electric P8.
One code base runs natively on **Windows, Linux and macOS** and in the **browser** (WebAssembly),
built with [egui / eframe](https://github.com/emilk/egui).

## Features

- **Projects with pages**: multi-line schematic pages (A3, 10 columns / 6 rows, title block),
  cover sheet and generated report pages. Page navigator with reordering.
- **IEC symbol library** (~40 symbols): coils and timers, NO/NC/changeover contacts, main
  contacts, push buttons, emergency stop, selector and proximity switches, fuses, circuit
  breakers, motor protection switch, overload relay, motors, lamps, power supply,
  transformer, terminals, PLC inputs/outputs, potentials, interruption points, R/C/diode.
- **Device-oriented engineering**: device tags (`-K1`, `-S3`, …) are assigned automatically.
  Main function (coil) + auxiliary functions (contacts) form one device. Select a coil and
  place contacts: they are assigned to it and numbered (13/14, 21/22, …).
- **Live cross references**: contacts show where their coil is (`/3.4`), coils show a
  contact image with all contacts and their locations, interruption points reference their
  counterparts on other pages.
- **Connectivity / potentials**: wires, T-junctions (with connection dots), terminals,
  potential connection points (L1, N, PE, +24V, …) and interruption points are evaluated
  into nets across pages. Selecting a wire highlights its whole potential.
- **Reports**: table of contents, device list, bill of materials (with parts and prices),
  terminal diagram, connection list, PLC assignment list, potential overview — in a window,
  as CSV export, or inserted as live report pages.
- **Project check**: aux functions without main function, duplicate main functions,
  open connection points, unpaired interruption points, short circuits between
  potentials, duplicate terminal numbers / PLC addresses, devices without part.
- **Parts management** and device data (part number, description).
- **Editing**: select / window select / move with rubber-band wires, rotate, mirror,
  copy / cut / paste (via clipboard, devices get new tags), undo / redo, texts, graphic
  lines and rectangles, grid snapping, pan and zoom.
- **Output**: SVG per page, printable HTML of all pages (print to PDF from the browser).
- **Storage**: JSON project files (`.oec`); the current project is also auto-saved
  (app storage on desktop, local storage in the browser).

## Build & run

Requires a recent stable Rust toolchain (`rustup update stable`).

### Desktop (Windows / Linux / macOS)

```bash
cargo run --release
```

On Linux you need the usual windowing / dialog libraries, e.g. on Debian/Ubuntu:

```bash
sudo apt install libxkbcommon-dev libgtk-3-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### Web (WebAssembly)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve          # development server on http://127.0.0.1:8080
trunk build --release  # static files in dist/ – host them on any web server
```

In the browser "Save" downloads the project file and "Open" uses the file picker.

### Tests

```bash
cargo test
```

## Usage

| Action | How |
| --- | --- |
| Place symbol | pick it in the library (right), click into the page; `R` rotate, `M` mirror, `Esc` stop |
| Draw wires | `W`, click start and corner points, ends automatically on a connection point; `Space` flips the corner |
| Select / move | click, `Shift`+click, drag on empty space for window selection, drag items to move |
| Edit properties | right panel: tag, function text, connection point designations, part, cross references |
| Pan / zoom | right or middle mouse drag / mouse wheel, `F` fits the page |
| Pages | `Page Up` / `Page Down`, navigator on the left |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` |
| Copy / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |

## Code structure

| File | Purpose |
| --- | --- |
| `src/model.rs` | Project data model (pages, symbols, wires, parts) and page layout constants |
| `src/symbols.rs` | Built-in IEC symbol library |
| `src/netlist.rs` | Connectivity analysis (nets / potentials, junctions) |
| `src/index.rs` | Device and cross-reference index |
| `src/reports.rs` | Reports and project check |
| `src/render.rs` | Backend independent page renderer (used by canvas and SVG) |
| `src/editor.rs` | Interactive canvas (tools, hit testing, pan/zoom) |
| `src/ops.rs` | Editing operations (placement defaults, move, rotate, paste, …) |
| `src/app.rs` | Application UI (menus, panels, dialogs) |
| `src/svg.rs` | SVG / print export |
| `src/io.rs` | Native file dialogs and browser up/downloads |
