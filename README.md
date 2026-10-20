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
- **Parts management** (EPLAN style):
  - a global **parts database** shared by all projects (product top group / group /
    subgroup tree, filter), with general, commercial (price, price unit, packaging,
    quantity unit), technical, mounting (dimensions, weight) data, accessories and a
    **function template** per part (e.g. coil A1/A2, main contacts, 13/14 NO);
  - **project parts**: parts are copied into the project when assigned; synchronize
    project ← database, transfer project → database, remove unused project parts;
  - **part selection** dialog with *device selection* (only parts providing the device's
    main function); several parts per device with quantity (contactor + aux block);
  - function templates drive **reserve contacts** in the contact image, "Apply
    designations" (connection point numbers from the part) and the project check
    (more functions placed than the parts provide, parts missing in the project);
  - import / export as CSV (`;` separated, header names) or JSON; the database is stored
    in the app storage (desktop) / local storage (browser).
- **Panel layout** (in the spirit of EPLAN Pro Panel):
  - enclosures (from a part such as `GEN.ENC-600X800` or custom size) with mounting plate;
  - mounting rails and cable ducts drawn by length; devices placed from a "devices to
    place" list with their real dimensions from the parts data (contactor + auxiliary
    block add up in depth, terminal strips are as wide as their terminals) and snapped
    onto rails; move (devices travel with their rail), rotate, delete;
  - 2D front view and a 3D view (orbit, pan, zoom, click to select);
  - collision and boundary checks (overlaps, outside the plate, too deep for the enclosure);
  - virtual wiring: every schematic connection between placed devices is routed through
    the cable duct network (shortest path) → wire list with lengths, wires shown in 2D/3D;
  - mounting layout pages (scaled 2D model view with dimensions) for printing / SVG;
    rails and ducts appear in the BOM by cut length, enclosures per piece;
  - jump between a device in the schematic and its place in the panel layout.
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

### Parts server

`openecraft-server` hosts a global parts database that all OpenECraft installations
(desktop and web) can share. It is a separate executable and runs independently of
the editor.

```bash
cargo run --release -p openecraft-server -- --bind 0.0.0.0:8090 --data parts-db.json --token <secret>
```

- The database is a JSON file, created with the example catalog if it does not exist,
  and written atomically on every change.
- Reading is open; changes need `Authorization: Bearer <token>`. Without `--token`
  (or the `OPENECRAFT_TOKEN` environment variable) the server is read-only.
- REST API (`/api/v1`): `GET /info`, `GET /parts?q=&group=&manufacturer=&offset=&limit=`,
  `GET|PUT|DELETE /parts/{number}`, `POST /import` (CSV or JSON), `GET /export.csv`.
- In the editor: *Project › Parts management › 🌐 Parts server* – enter the server URL,
  test the connection, download all parts into the local database or upload the
  selected part / the whole local database (needs the access token).
- The server speaks plain HTTP; put it behind a TLS reverse proxy when it is reachable
  from outside your network.

### Tests

```bash
cargo test --workspace
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
| `crates/openecraft-parts` | Shared library: part model, function templates, CSV/JSON, catalog, server API types |
| `server/` | `openecraft-server`: parts database server (REST API, JSON storage) |
| `src/parts_server.rs` | Client for the parts server |
| `src/model.rs` | Project data model (pages, symbols, wires, parts) and page layout constants |
| `src/symbols.rs` | Built-in IEC symbol library |
| `src/netlist.rs` | Connectivity analysis (nets / potentials, junctions) |
| `src/index.rs` | Device and cross-reference index |
| `src/reports.rs` | Reports and project check |
| `src/render.rs` | Backend independent page renderer (used by canvas and SVG) |
| `src/editor.rs` | Interactive canvas (tools, hit testing, pan/zoom) |
| `src/ops.rs` | Editing operations (placement defaults, move, rotate, paste, …) |
| `src/panel.rs` | Panel layout model, device sizes, rail snapping, collisions, duct routing |
| `src/panel_ui.rs` | Panel layout workspace: navigator, 2D editor, 3D view, properties |
| `src/parts.rs` | Parts database, function templates, project parts, CSV import/export |
| `src/parts_ui.rs` | Parts management window and part selection dialog |
| `src/app.rs` | Application UI (menus, panels, dialogs) |
| `src/svg.rs` | SVG / print export |
| `src/io.rs` | Native file dialogs and browser up/downloads |
