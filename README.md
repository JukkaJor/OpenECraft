# OpenECraft

An open source electrical CAE editor written in Rust, in the spirit of EPLAN Electric P8.
One code base runs natively on **Windows, Linux and macOS** and in the **browser** (WebAssembly),
built with [egui / eframe](https://github.com/emilk/egui).

## Features

- **Languages**: English, German, French and Spanish (View › Language; the editor and the
  parts manager remember the choice and start in the system / browser language if supported).
- **Projects with pages**: multi-line schematic pages (A3, 10 columns / 6 rows, title block),
  cover sheet and generated report pages. Page navigator with reordering.
- **IEC symbol library** (~40 symbols): coils and timers, NO/NC/changeover contacts, main
  contacts, push buttons, emergency stop, selector and proximity switches, fuses, circuit
  breakers, motor protection switch, overload relay, motors, lamps, power supply,
  transformer, terminals, PLC inputs/outputs, potentials, interruption points, R/C/diode.
- **Device-oriented engineering**: device tags (`-K1`, `-S3`, …) are assigned automatically.
  Main function (coil) + auxiliary functions (contacts) form one device. Select a coil and
  place contacts: they are assigned to it and numbered (13/14, 21/22, …).
- **Structure identifiers (IEC 81346)**: pages carry a function (`=A1`) and a location
  (`+CAB1`). Devices placed on a page get its structure (`=A1+CAB1-K1`); on the page, tags
  and cross references show only what differs from the page structure (EPLAN style),
  tags are numbered per structure, copy / paste moves devices into the target structure,
  changing a page structure can move its devices along. The device navigator groups by
  structure; the title block shows the full page name (`=A1+CAB1/3`).
- **Cables**: cable definitions (tag, type, cores, cross-section, length, part from the
  product group *Cables*). Wires are assigned to cores (several wires at once get the free
  cores in order); cores and the cable tag are shown on the schematic. Cable overview and
  cable diagram reports, cables in the BOM by length, checks for unknown cables, cores
  used twice or not in the cable.
- **Wire numbers**: one number per connection (all its wire segments; the two sides of a
  terminal are separate connections). Automatic numbering: sequential, page/column
  (`3.4.1`) or by potential name (`L1`, `+24V`), with prefix and start value; manually
  set numbers are kept. Shown on the schematic, in the connection list and the panel
  wire list; checks for conflicting and duplicate numbers.
- **Macros**: save a selection (window macro) or a whole page (page macro) into a macro
  library (stored with the application, JSON import / export). Window macros are placed
  with the mouse (ghost preview, devices get new tags), page macros insert a page.
  Placeholders such as `{MOTOR}` in tags, texts, function texts and designations are
  filled from **value sets** when inserting.
- **Terminal strip editor**: terminal order (move up / down, sort), renumbering 1 … n,
  **jumpers** (bridged terminals share their potential), spare terminals (in the terminal
  diagram and the BOM).
- **PLC overview**: all inputs / outputs per card with connected devices, editable
  addresses and function texts, card start byte / rack / slot, readdressing, **symbol
  table export / import** (CSV: name, data type, address, comment) for PLC programming
  tools; addresses used on several cards are reported.
- **Revisions**: a revision stores a snapshot of all pages and can release (lock) them;
  locked pages are read only. Changes since the last revision (added / changed / removed
  symbols, wires, texts, pages) are listed, marked on the pages with a revision triangle
  and reported in the *revision overview*.
- **Symbol editor and libraries**: besides the IEC symbols there are NFPA / JIC
  symbols, fluid power symbols (ISO 1219: pump, cylinder, 4/2 valve, filter, gauge,
  tank), single-line / overview symbols (cross references to the device, not part of
  the netlist) and black boxes / PLC boxes. Own symbols are drawn in the symbol editor
  (lines, rectangles, circles, texts, tag / label fields, connection points on the 4 mm
  grid, function such as main function, contact, terminal, PLC point), or copied from a
  built-in symbol; they are stored in the project and exchanged as library files.
- **Forms**: title blocks are lists of lines, rectangles and texts with placeholders
  (`{project}`, `{page}`, `{sheet}`, `{rev.date}`, user properties `{prop:Name}` …),
  edited in the title block editor and shared as files; one per project with an
  override per page (built-in: Standard, Compact, None). Report forms choose the
  columns, their order, the heading and the rows per report page.
- **Live cross references**: contacts show where their coil is (`/3.4`), coils show a
  contact image with all contacts and their locations, interruption points reference their
  counterparts on other pages.
- **Connectivity / potentials**: wires, T-junctions (with connection dots), terminals,
  potential connection points (L1, N, PE, +24V, …) and interruption points are evaluated
  into nets across pages. Selecting a wire highlights its whole potential.
- **Reports**: table of contents, device list, bill of materials (with parts and prices),
  terminal diagram, connection list, PLC assignment list, potential overview, cable
  overview, cable diagram — in a window,
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
  - **connection points** per part (designation, position in the front view, the
    direction the wire leaves); parts without their own get them from the function
    templates by the IEC convention (odd designations at the top, even at the bottom,
    door devices from the rear, PLC modules at the bottom). Edited in the part editor
    (tab *Connection points*: table, click into the front view to place one); the
    panel layout wires to them; CSV column `Connection points`: `A1:22.5/0/55/up|A2:…`;
  - import / export as CSV (`;` separated, header names) or JSON; the database is stored
    in the app storage (desktop) / local storage (browser).
- **Panel layout** (in the spirit of EPLAN Pro Panel):
  - enclosures (from a part such as `GEN.ENC-600X800` or custom size) with mounting plate;
  - mounting rails and cable ducts drawn by length; devices placed from a "devices to
    place" list with their real dimensions from the parts data (contactor + auxiliary
    block add up in depth, terminal strips are as wide as their terminals) and snapped
    onto rails; move (devices travel with their rail), rotate, delete;
  - 2D front view and a 3D view (orbit, pan, zoom, click to select);
  - collision and boundary checks (overlaps, outside the plate, too deep for the enclosure
    including a door clearance, mounted over a cutout, inside the clearance a device
    needs above and below from its part data `Clearance: 25 mm`); where a device has a
    3D model, the **models** are checked for penetration, not just their boxes;
  - **enclosure door**: command and signal devices (pushbuttons, lamps …) are placed on
    the door (front view *Door*); each gets its **door cutout** (Ø22.5 by default or from
    the part data `Door cutout: Ø30.5` / `92 x 68`), checked against the door frame and
    in 3D against the equipment of the mounting plate; their wires are routed over the
    hinge; the mounting layout page shows plate and door side by side;
  - copper **busbar systems** (bars drawn, metres and holders in the BOM) and rectangular
    or round **cutouts** in the mounting plate;
  - **drilling pattern**: rails, ducts and busbar holders get tapped holes along their
    length, devices screwed onto the plate get holes from their part data
    (`Mounting holes: M5: 9,12; 69,72`) or two default holes; **NC export** per
    enclosure and face as DXF (outline, one layer per hole diameter, cutouts) or CSV;
  - virtual wiring: every schematic connection between placed devices is routed from
    its connection point (rotated devices, aux blocks, terminal by terminal) into the
    duct the terminal faces and through the cable duct network (shortest path) → wire
    list with lengths, wires shown in 2D/3D;
  - **duct fill levels**: cross sections of the routed wires (with insulation) against
    the duct cross section at its fullest stretch; ducts coloured green / orange / red
    in the front view, fill level in the duct properties, limit in the enclosure
    properties (70 % by default); report *Duct fill levels*, the check warns above the
    limit and hints above 80 % of it; when a duct is overfull, the wiring is routed
    again, thickest wires first, and wires take a detour (up to 10 × longer) around
    stretches they would fill above the limit; the selected device shows its
    connection points;
  - mounting layout pages (scaled 2D model view with dimensions) for printing / SVG;
    rails and ducts appear in the BOM by cut length, enclosures per piece;
  - jump between a device in the schematic and its place in the panel layout.
- **Editing**: select / window select / move with rubber-band wires, rotate, mirror,
  copy / cut / paste (via clipboard, devices get new tags), undo / redo, texts, graphic
  lines and rectangles, grid snapping, pan and zoom.
- **Output / data exchange**: **PDF** of all pages (bookmarks per page, clickable cross
  references and report rows), **DXF** (R12, layers by kind) per page or all pages side
  by side, SVG per page, printable HTML, production wire list (CSV with wire numbers,
  ends, colour, cross-section, routed length, stripping length and ferrule per
  DIN 46228-4, cable cores; panel wires sorted by cross-section, colour and length for
  wire processing machines; ferrules also in the BOM), report CSVs, PLC symbol tables,
  NC data of mounting plates.
  DWG and EPLAN's EDZ format are proprietary and not supported.
- **Wire harnesses**: single point-to-point connections (`-S1:13 – -K1:A1`) are
  assigned to harnesses (`-WH1`), independent of the other connections of the same
  potential section; assignments follow renamed devices, missing connections are
  reported by the check; the *harness data*
  report lists their wires with length (routed in the panel layout or the harness's
  field length) and end treatment, the harness window shows the connectors with their
  pin assignment and the bundle diameter; export as simplified **KBL** (VDA 4964, XML)
  for harness manufacturing.
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

### Parts manager

`openecraft-parts-manager` is a standalone application that maintains the parts of a
parts server: create new parts, edit existing ones (including 2D / 3D models), rename
and delete parts, upload CSV / JSON parts files and export all parts. It uses the same
part editor as the editor's parts management and runs on the desktop and in the browser.

```bash
cargo run --release -p openecraft-parts-manager        # desktop
cd parts-manager && trunk serve                        # web, http://127.0.0.1:8082
cd parts-manager && trunk build --release              # static files in parts-manager/dist/
```

Enter the server URL and the access token (kept in memory only) and click *Connect*.
Changes are saved to the server with *Save* (`Ctrl+S`); the editor asks before unsaved
changes are discarded and before a part is deleted.

### Command line and MCP server (AI agents)

`openecraft-cli` edits projects and maintains the parts server without a window. The same
commands are offered to AI agents over the [Model Context Protocol](https://modelcontextprotocol.io)
(`openecraft-cli mcp`, JSON-RPC on stdin / stdout), so an agent can create projects, place
symbols, draw wires between connection points, assign parts, run the check, generate
reports and export PDF / SVG / DXF.

```bash
cargo run --release -p openecraft-cli -- help                 # all commands
cargo run --release -p openecraft-cli -- help connect         # parameters of a command
openecraft-cli new_project --name "Pump control" --path pump.oec
openecraft-cli -p pump.oec place_symbol --page 1 --symbol pb_no --x 200 --y 100   # -S1
openecraft-cli -p pump.oec place_symbol --page 1 --symbol coil --x 200 --y 160    # -K1
openecraft-cli -p pump.oec connect --page 1 --from -S1:14 --to -K1:A1
openecraft-cli -p pump.oec check
openecraft-cli -p pump.oec export --format pdf --path pump.pdf
openecraft-cli --server http://127.0.0.1:8090 --token <secret> parts_import --path parts.csv
```

- On the command line every change is written back to `--project` at once. In an MCP
  session the project stays open in memory: `save_project` writes it, `undo` / `redo`
  revert single tool calls.
- Parameters are `--name value` (also `--name=value`, or all at once with `--json '{…}'`);
  lists as `--ids 3,4` and points as `--points "0,0 40,0"`.
- Parts server: `--server` / `--token`, the `OPENECRAFT_SERVER` / `OPENECRAFT_TOKEN`
  environment variables or the `parts_connect` tool; default `http://127.0.0.1:8090`.
- Help and messages follow `--lang` or the system language; the MCP server answers in
  English unless `--lang` is given.

Register the MCP server with an agent, e.g. Claude Code:

```bash
claude mcp add openecraft -- /path/to/openecraft-cli mcp
```

or in the `mcpServers` configuration of other clients:

```json
{ "mcpServers": { "openecraft": { "command": "openecraft-cli", "args": ["mcp"] } } }
```

The agent can read and write any file the server process can access; start it in a
working directory (and with a parts server token) you are fine with the agent using.

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
| Page structure | *Page › Page properties…*: function (`=`) and location (`+`) |
| Cables | *Project › Cables…*; select wires › *Assign to cable*; single wire: cable and core |
| Wire numbers | *Project › Wire numbering…*; single wire: type a number (kept by automatic numbering) |
| Macros | select items › *Edit › Save selection as macro…*; *Insert › Macro…* to place one |
| Terminal strips / PLC | *Project › Terminal strips…* / *Project › PLC overview…* |
| Revisions | *Project › Revisions…*: create (and release) revisions, lock / unlock pages, see changes |
| Export | *File*: PDF, DXF (page / all), SVG, HTML, production wire list |
| Own symbols | *Project › Symbol editor…*; they appear under *User symbols* in the symbol list |
| Title blocks / report forms | *Project › Title blocks and report forms…*; per page in the page properties |
| Panel production | panel layout: *Busbar* / *Cutout* tools; enclosure properties: NC data (DXF / CSV) |
| Door devices | panel layout: front view *Door*, place devices there or move them in their properties |
| Harnesses | *Project › Harnesses…*; select the wires between two devices › *Assign to harness*, or per connection in the wire properties; KBL export in the window |
| Pan / zoom | right or middle mouse drag / mouse wheel, `F` fits the page |
| Pages | `Page Up` / `Page Down`, navigator on the left |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` |
| Copy / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |

## Code structure

| File | Purpose |
| --- | --- |
| `crates/openecraft-i18n` | Languages and UI translations (English, German, French, Spanish); catalog in `src/catalog/*.tsv` |
| `crates/openecraft-core` | Headless project model shared by the editor and the command line: pages, devices, netlist, checks, reports, export |
| `crates/openecraft-parts` | Shared library: part model, function templates, CSV/JSON, catalog, server API types, 3D math |
| `crates/openecraft-client` | Requests to the parts server and decoding of its answers, shared by the applications and the command line |
| `crates/openecraft-ui` | Shared egui library: part editor and previews, parts server client, file dialogs |
| `server/` | `openecraft-server`: parts database server (REST API, JSON storage) |
| `cli/` | `openecraft-cli`: command line and MCP server; one tool list (`src/tools/`) for both |
| `parts-manager/` | `openecraft-parts-manager`: standalone parts server maintenance (desktop + web) |
| `crates/openecraft-core/src/model.rs` | Project data model (pages, symbols, wires, parts) and page layout constants |
| `crates/openecraft-core/src/symbols.rs` | Built-in symbol libraries (IEC, NFPA, fluid power, single-line, black boxes) |
| `crates/openecraft-core/src/symlib.rs` | User symbols: registry, editing helpers, library files |
| `crates/openecraft-core/src/forms.rs` | Title blocks (placeholders) and report forms |
| `crates/openecraft-core/src/production.rs` | Panel production: drilling pattern, door cutouts, NC export, wire end treatment, clearances |
| `crates/openecraft-core/src/harness.rs` | Wire harnesses: wires, connectors, bundle diameter, KBL export |
| `crates/openecraft-core/src/netlist.rs` | Connectivity analysis (nets / potentials, junctions) |
| `crates/openecraft-core/src/index.rs` | Device and cross-reference index |
| `crates/openecraft-core/src/reports.rs` | Reports and project check |
| `crates/openecraft-core/src/render.rs` | Backend independent page renderer (used by canvas and SVG) |
| `src/editor.rs` | Interactive canvas (tools, hit testing, pan/zoom) |
| `crates/openecraft-core/src/ops.rs` | Editing operations (placement defaults, move, rotate, paste, cable cores, page structure, …) |
| `crates/openecraft-core/src/structure.rs` | Structure identifiers: parse / complete / shorten tags (`=A1+CAB1-K1`) |
| `crates/openecraft-core/src/wirenum.rs` | Wire numbers per connection and automatic numbering |
| `crates/openecraft-core/src/macros.rs` | Window / page macros, placeholders and value sets |
| `crates/openecraft-core/src/strips.rs` | Terminal strips: order, renumbering, jumpers, spare terminals |
| `crates/openecraft-core/src/plc.rs` | PLC points, addressing, symbol table export / import |
| `crates/openecraft-core/src/revision.rs` | Revisions: page snapshots, change detection, revision names |
| `crates/openecraft-core/src/pdf.rs` / `dxf.rs` | PDF and DXF export (through the `render::Draw` trait) |
| `src/app/engineering.rs` | Windows for macros, terminal strips, PLC overview and revisions |
| `src/app/symbol_editor.rs` / `src/app/forms_ui.rs` | Symbol editor and forms (title block / report form) windows |
| `src/app/harness_ui.rs` | Harness window |
| `crates/openecraft-core/src/panel.rs` | Panel layout model, device sizes, rail snapping, collisions, duct routing |
| `src/panel_ui.rs` | Panel layout workspace: navigator, 2D editor, 3D view, properties |
| `crates/openecraft-core/src/parts.rs` | Parts database, function templates, project parts, CSV import/export |
| `src/parts_ui.rs` | Parts management window, part selection dialog, parts server panel |
| `src/app.rs` | Application UI (menus, panels, dialogs) |
| `crates/openecraft-core/src/svg.rs` | SVG / print export |
| `src/io.rs` | File kinds of the editor (dialogs are in `openecraft-ui`) |
