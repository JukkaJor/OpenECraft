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
- **Potential definitions** (*Project › Potentials…*): kind (L, N, PE, DC +/−, signal),
  voltage, wire colour (IEC 60757 codes) and cross-section per potential; presets
  IEC 60204-1 (machinery, new projects) and IEC 60445 (phase colours). Wires without
  their own colour / cross-section take them from the potential of their connection
  (also behind terminals), otherwise from the project default for connections without
  a defined potential; cable cores keep the cable's values. The wire properties show
  the automatic value and where it comes from; the wire lists, panel wiring, duct fill,
  ferrules, harnesses and marking use it. The check reports GNYE outside protective
  conductors and PE in other colours, manual values that differ from the definition
  and potentials without definition. *View › Wire colours* (saved with the project)
  draws the wires in their colour on the schematic and in PDF / SVG export; two-colour
  wires such as GNYE are drawn dashed in their second colour.
- **Wire numbers**: one number per connection (all its wire segments; the two sides of a
  terminal are separate connections). Automatic numbering: sequential, page/column
  (`3.4.1`) or by potential name (`L1`, `+24V`), with prefix and start value; manually
  set numbers are kept. Shown on the schematic, in the connection list and the panel
  wire list; checks for conflicting and duplicate numbers.
- **Macros**: save a selection (window macro) or a whole page (page macro) into a macro
  library (stored with the application, JSON import / export). Window macros are placed
  with the mouse (ghost preview, devices get new tags), page macros insert a page.
  Placeholders such as `{MOTOR}` in tags, texts, function texts and designations are
  filled from **value sets** when inserting. **Variants**: a macro can hold the same
  function in several versions (A, B, … – e.g. direct on line and star-delta); save a
  selection under the name of an existing macro with another variant, choose the
  variant when inserting.
- **Configurator** (*Project › Configurator…*): options with their choices (e.g.
  `STARTER` = DOL / SD) and rules that insert a page macro in a variant with
  placeholder values (`NAME=Pump; POWER={POWER}`) when their condition holds
  (`STARTER=DOL & LINE!=2`, `LINE=1|2`). *Generate pages* replaces the pages generated
  before (marked in the page properties, switch off to keep one); option values fill
  `{OPTION}` placeholders; problems (unknown macro, variant or option) stop it.
- **Terminal strip editor**: terminal order (move up / down, sort), renumbering 1 … n,

  **jumpers** (bridged terminals share their potential), spare terminals (in the terminal

  diagram and the BOM);

  every terminal has its part: PE terminals take the strip's part with the tech data
  `Terminal type: PE`, all others its feed-through part (bill of materials, panel
  length of the strip, connection points per terminal); **automatic accessories**
  from the tech data of the terminal parts (`End bracket: …`, `End plate: …`,
  `Jumper: …` with jumper parts `Poles: n`): an end bracket at both ends, an end
  plate after the last terminal of each part, plug-in jumpers of the shortest
  fitting length (cut if longer, teeth broken off at terminals in between) – in the
  bill of materials, the terminal diagram, the panel layout (2D / 3D) and the check;
  the terminal strip window shows the strip on the rail with its jumpers and has a
  switch for the automatic accessories.
- **Plugs** (*Project › Plugs…*, symbol group *Plugs*): pin and socket (mated pair, two
  connection points), male pin and female socket (one each); tag = plug (`-X10`),
  designation = pin. Male and female pins of the same designation are mated: the
  potential passes through, the two sides are separate connections (wire numbers,
  cables). Pins are numbered per plug when placed; the plug window and the *plug
  diagram* report list the pins in plug order with the socket side, the pin side and
  the potential; parts with `Plug pin` functions give the pin count (unused pins are
  listed as spare). The check reports pins placed twice and more pins than the plug
  parts have; KBL export names the housing part of plug connectors.
- **Marking** (*Project › Marking…*: wire marker text, material per kind with piece
  counts and export; *File › Export marking (CSV)…*, report *Marking*): device markers (legend
  plates for door devices; the enclosure's location left out of the tag), terminal markers
  in rail order with the strip marker, wire markers (wire number on both ends, or
  destination marking: each end shows the other end), cable markers; grouped by
  marker material in the order of application, as CSV for marking software (UTF-8,
  `;`, header `Material;Category;Text;Count;Enclosure;Group`); the material is in the
  bill of materials. Texts and materials are project settings (`marking`).
- **PLC overview**: all inputs / outputs per card with connected devices, editable
  addresses and function texts, card start byte / rack / slot, readdressing, **symbol
  table export / import** (CSV: name, data type, address, comment) for PLC programming
  tools; addresses used on several cards are reported.
- **Revisions**: a revision stores a snapshot of all pages and can release (lock) them;
  locked pages are read only. Changes since the last released revision (added / changed /
  removed symbols, wires, texts, pages) are listed, marked on the pages with a revision
  triangle and reported in the *revision overview*.
- **Release workflow**: with approvers defined (*Project › Revisions… › Approval
  workflow*: user names and the number of approvals needed) a new revision starts as a
  draft, is submitted for review, approved or rejected by the approvers (recorded with
  user, date and note) and released once approved. Pages changed after the submission
  block the release until the revision is submitted again (earlier approvals become void).
- **Baselines and compare / merge**: a baseline stores the whole project (pages, devices,
  parts, cables, panel layout, settings); releasing a revision stores one automatically.
  *Project › Compare and baselines…* compares the project with a baseline, the pages of a
  revision or another project file: added / removed / changed pages (with the changed
  items), devices, parts, potentials, cables, harnesses, enclosures, terminal strips,
  plugs, PLC cards and settings sections. Selected differences are taken over.
- **Review: comments and redlining**: comments on a page position with status (open, in
  progress, resolved, rejected), author, date and replies; redline (to be changed) and
  greenline (changed as built) markup as line, rectangle, revision cloud or text. They are
  drawn on a layer of their own (shown on screen, not in exports), listed in the *Review*
  tab of the navigator and can be added to released pages and to pages another user has
  locked. On the project server they are merged record by record without locks.
- **Change requests**: requests from inside or outside the project (number, title,
  description, requester, origin, priority, status, affected pages and devices,
  implementing revision) with linked comments (*Project › Change requests…*). Exchange
  with other systems as CSV (`Number;Title;Description;Requester;Origin;Date;Priority;
  Status;Pages;Devices;Revision`) or JSON; importing updates requests with a known number.
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
  - **graphics** per part: 2D front view (SVG) and 3D model (STL, OBJ, **STEP**:
    solids and surface models of AP203 / AP214 / AP242 files, placed by their
    assembly structure, tessellated and converted to mm; **IGES 5.3**: solids and
    trimmed surfaces; **IFC**: tessellated, faceted and extruded bodies of all
    products, placed and converted to mm; **JT**: the tri-strip meshes of JT 8 files);
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
  - import / export as CSV (`;` separated, header names) or JSON, import of **BMEcat**
    catalogs (1.2 / 2005, as published by manufacturers and wholesalers, e.g. from the
    EPLAN Data Portal or ETIM / eCl@ss data pools): identification, description, price,
    data sheet, the ETIM / eCl@ss class and every feature as technical data;
    dimensions, weight, rated current / voltage and power loss go into their fields
    (by feature name, or the ETIM codes of width, height and depth); the database is
    stored in the app storage (desktop) / local storage (browser).
- **Panel layout** (in the spirit of EPLAN Pro Panel):
  - enclosures (from a part such as `GEN.ENC-600X800` or custom size) with mounting plate;
  - mounting rails and cable ducts drawn by length; devices placed from a "devices to
    place" list with their real dimensions from the parts data (contactor + auxiliary
    block add up in depth, terminal strips are as long as their terminals and spares) and snapped
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
  - jump between a device in the schematic and its place in the panel layout;
  - **climate** (IEC/TR 60890, enclosure properties): power loss of the placed devices
    from the tech data of their parts (`Power loss: 7.5 W`) plus an additional loss,
    effective surface by installation type (single, in a suite, free-standing, wall
    mounted, roof covered), material (heat transfer coefficient), ambient and allowed
    inside temperatures → inside temperature, filter fan air flow or cooling unit
    output, heater output; report *Enclosure climate*, the check warns when cooling is
    needed.
  - **line-ups**: enclosures bayed side by side with cable passages between them
    (cutouts in the facing side walls, or sides open to each other); the 3D view shows
    the whole line-up, wires run from one enclosure into the next;
  - **3D routing** of cables, tubes and (optionally) wires through the line-up: in cable
    ducts, along routing paths drawn over the mounting plate, or freely around the
    devices; segregation of the classes power, control, signal and fluid, routing
    reservations, bend radii, cable bundles with their branches, shield terminations
    (shield clamp rail, pigtail) and their checks; routes can be frozen for production
    and stay as they are until they become invalid;
- **Electrical sizing** (IEC 60204-1): the wires behind a protective device – from its
  load side through main contacts and overload relays – must carry its rated current
  (part `Current`, the upper value of a setting range; current-carrying capacity per
  table 6, method B2); cables get their voltage drop to the load with the highest rated
  current at their ends (√3 for three cores, copper at operating temperature), report
  *Cable sizing*; the check reports too thin wires and cables (errors) and voltage
  drops above 5 %.
- **Editing in tables** (*Project › Edit in table…*): devices (tag, function text,
  description, part), connections (wire number, colour, cross-section, cable, core) and
  terminals (strip, number, function text) as rows; filter, edit cells in place, fill a
  column or find and replace in the filtered rows; CSV export / import for changes in a
  spreadsheet (rows by key, only changed cells are applied); rows on released pages
  stay unchanged. Renaming a device moves its strip, plug and PLC card data along.
- **Editing**: select / window select / move with rubber-band wires, rotate, mirror,
  copy / cut / paste (via clipboard, devices get new tags), undo / redo, texts, grid
  snapping, pan and zoom.
- **Schematic graphics**: lines, rectangles, circles, ellipses, arcs, polylines and
  polygons (width, colour, dashed, filled), dimensions (aligned / horizontal / vertical,
  scale, unit), PNG / JPEG images (also in the PDF and SVG export). **Layers**
  (navigator › Layers): visible, locked, printed, colour; DXF export per layer.
- **Connection and cable definitions**: a connection definition point on a wire sets
  number, colour and cross-section of the whole connection and the core of its wire; a
  cable definition line drawn across wires makes them cores of a cable (new cable with
  numbered cores, or a chosen one). The check reports points beside wires, contradicting
  or not applied points, and lines of unknown cables.
- **Routing**: auto-connect draws wires between facing free connection points when
  symbols are placed or moved; the wire tool can route around symbols and other wires
  (A); collinear wire pieces can be joined. Also as `route` / `autoconnect` commands.
- **Property placement**: drag the tag block of a selected symbol; place further device
  properties (description, part / type number, manufacturer, designation, connection
  points, structure) beside it.
- **Navigators**: pages, devices (select on page, place further functions, rename),
  connections (filter, show, select, add a connection definition), cables (cores in
  use and free, cable definitions, draw a cable definition), layers and the check.
- **Output / data exchange**: **PDF** of all pages (bookmarks per page, clickable cross
  references and report rows), **DXF** (R12, layers by kind) per page or all pages side
  by side, SVG per page, printable HTML, production wire list (CSV with wire numbers,
  ends, colour, cross-section, routed length, stripping length and ferrule per
  DIN 46228-4, cable cores; panel wires sorted by cross-section, colour and length for
  wire processing machines; ferrules also in the BOM), report CSVs, PLC symbol tables,
  NC data of mounting plates, the **panel layout in 3D** as **glTF 2.0** (viewers, web,
  PLM; metres, one node per enclosure and item, colours), **STEP AP214** (MCAD;
  faceted solids with colours, millimetres, z up), **IGES 5.3** (MCAD; B-rep solids
  with planar faces, open shells for non-closed meshes, colours and names), **IFC4**
  (BIM; a distribution board per enclosure with its components as triangulated
  elements) and **JT** (PLM viewers; tri-strip meshes per item, JT 8.1 layout) –
  walls, door, mounting plate, rails, ducts, busbars and devices with the 3D models of
  their parts.
  **DXF import** (*File › Import DXF as page…*): old drawings as graphics on a new
  schematic page – lines, polylines, circles, arcs, ellipses, splines, texts, MTEXT,
  block references and dimensions, in their units, scaled down to the drawing area
  when they are larger.
  DWG and EPLAN's EDZ format are proprietary and not supported.
- **Wire harnesses**: single point-to-point connections (`-S1:13 – -K1:A1`) are
  assigned to harnesses (`-WH1`), independent of the other connections of the same
  potential section; assignments follow renamed devices, missing connections are
  reported by the check; the *harness data*
  report lists their wires with length (routed in the panel layout or the harness's
  field length) and end treatment, the harness window shows the connectors with their
  pin assignment and the bundle diameter; export as simplified **KBL** (VDA 4964, XML)
  for harness manufacturing.
- **Wiring assistant** (in the spirit of EPLAN Smart Wiring): the *Wiring* workspace
  turns the routed wires of the panel layout into a touch-friendly checklist for the
  workshop – ordered by device, position, cross-section or wire number – with the
  current wire highlighted in 3D, its colour, cross-section, length, stripping length
  and ferrule, and the progress in wires and metres. Wires are marked as wired or as a
  problem (with a note); marks are stored in the project, so wires changed in the
  schematic after they were wired are flagged by the check. The web build opens the
  assistant directly with `…/#wiring` (desktop: `--wiring`), which makes a tablet in
  the workshop a cheap wiring station; checklist with state as CSV.
- **Fluid and process**: wires between the ports of fluid power and P&ID symbols are
  hoses / pipes in their own nets (not mixed with electrical potentials; a fluid port
  on an electrical net is an error), numbered apart from the wires (`H1`, `H2` …),
  with medium, nominal size, colour and length, listed in the *hose list*.
  - *Fluid netlist*: every port gets its function from its ISO 1219 / ISO 5599
    designation (supply, pressure P/1, working A/B/2/4, exhaust R/S/3/5, return T,
    pilot X/Y/Z/12/14, leakage L, in-line) and every hose line its kind (pressure,
    working, return / exhaust, pilot … line) with its branches (tees).
  - *Hose configuration*: per hose, tube or pipe the line type, part (by the metre),
    end fittings, maximum pressure and bend radius; the cut length adds the project's
    cutting allowance per end. Tubes between devices placed on the same mounting
    plate are routed through its ducts like wires, which gives the length when none
    is set. The *tubing / piping list* sums pieces, cut lengths, bends and fittings
    per type, size and part.
  - *Fluid device groups*: valve terminals, service units, manifolds and power units
    with their devices in slot order and the number of places.
  - *Electro-fluid*: fluid devices are linked with the electrical devices that actuate
    or monitor them (solenoid coils, position and pressure switches); both schematics
    show cross references to each other, and *Synchronize* links solenoid valves with
    the coil of the same number (`-QM3` → `-MB3`) and copies function texts.
  - Checks: pressure lines on a return / exhaust, different media or sizes on one hose
    or one device, hoses rated below the operating pressure or shorter than their
    route, solenoid valves without coil, coils without valve, wrong or missing links,
    full or duplicated groups; open exhausts of pneumatic devices vent to the
    atmosphere instead of being open connection points.
  - Reports: *fluid netlist*, *hose configuration*, *tubing / piping list*, *fluid
    device groups*, *electro-fluid cross reference*. P&ID
  symbols (ISO 10628: vessel, pumps, valves, heat exchanger, instruments) support
  pre-planning: instruments carry function letters and loop number (`TIC 101`), the
  PCT loops of the pre-planning (*Project › Fluid and process…*) list their devices,
  and the *PCT loop overview* shows whether each loop is planned, in engineering or
  engineered in the schematic.
- **Project texts in several languages**: function texts, page descriptions, free
  texts and descriptions are written in a source language; the project's translation
  dictionary (*Project › Project languages…*) translates them line by line into any
  number of target languages, with CSV export / import for translators and a check
  for untranslated texts. *File › Export as PDF in language* (command line: `export`
  with `language`) puts out the documentation per language – project texts from the
  dictionary, reports and title blocks in that language (English, German, French,
  Spanish; other languages get English report headings).
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

`openecraft-parts-server` hosts a global parts database that all OpenECraft installations
(desktop and web) can share. It is a separate executable and runs independently of
the editor.

```bash
cargo run --release -p openecraft-parts-server -- --bind 0.0.0.0:8090 --data parts-db.json --token <secret>
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

### Project server (several users)

`openecraft-project-server` stores projects for a team. Several users open the same
project at the same time and work on different pages; a page is edited by one user at a
time.

```bash
cargo run --release -p openecraft-project-server -- --bind 0.0.0.0:8091 --data projects --token <secret>
```

- Each project is stored as `<name>.oec` (a normal project file) next to
  `<name>.state.json` (revision, id counter) in the data directory.
- With `--token` (or `OPENECRAFT_PROJECT_TOKEN`) every request needs
  `Authorization: Bearer <token>`; without it everyone who reaches the server can
  change its projects.
- Locks: every page and the project data (page list and order, devices, parts, panel
  layout, settings) are locked separately. Editing something you have not locked
  requests the lock automatically; if another user holds it, the change is taken back.
  Pages locked by others are read-only and marked with 🔒 and the user's name in the
  page list. Locks end when they are released, when the project is closed, or 90 s after
  the editor stopped sending its status (every 15 s).
- Saving merges only what you have locked into the server's project; everything else
  comes from the server. Changes of other users are loaded automatically.
- Each session gets its own range of object ids, so users never create the same id.
- In the editor: *File › Project server…* – enter the server URL, your user name and the
  token, list the projects and open one, or upload the current project. While a shared
  project is open, *Save* (`Ctrl+S`) saves to the server; the window and the page list's
  context menu lock and release pages.
- REST API (`/api/v1`): `GET /info`, `GET|POST /projects`, `GET|PUT /projects/{name}`,
  `POST /projects/{name}/open|status|lock|release|close` (see `openecraft_core::collab`).

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
cargo clippy --workspace --all-targets
cargo clippy -p openecraft -p openecraft-parts-manager --target wasm32-unknown-unknown
```

All three must pass without warnings (CI runs them with `-D warnings`).

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
| Configurator | *Project › Configurator…*: options, rules with page macros, *Generate pages* |
| Bulk changes | *Project › Edit in table…*: filter, edit, fill, find and replace, CSV |
| Enclosure climate | panel layout: select the enclosure › *Climate (IEC 60890)* |
| Terminal strips / PLC | *Project › Terminal strips…* / *Project › PLC overview…* |
| Plugs | place pins from the symbol group *Plugs*; *Project › Plugs…* for pin order and both sides |
| Revisions | *Project › Revisions…*: create (and release) revisions, approval workflow, lock / unlock pages, see changes |
| Compare | *Project › Compare and baselines…*: create baselines, compare, take differences over |
| Review | *Review* tab of the navigator, 💬 *Comment* and redline / greenline tools of the toolbar |
| Change requests | *Project › Change requests…*: requests, CSV / JSON import and export |
| Export | *File*: PDF, DXF (page / all), SVG, HTML, production wire list, panel layout in 3D (glTF / STEP / IGES / IFC / JT) |
| Import old drawings | *File › Import DXF as page…* |
| Own symbols | *Project › Symbol editor…*; they appear under *User symbols* in the symbol list |
| Title blocks / report forms | *Project › Title blocks and report forms…*; per page in the page properties |
| Panel production | panel layout: *Busbar* / *Cutout* tools; enclosure properties: NC data (DXF / CSV) |
| Door devices | panel layout: front view *Door (front)* for door devices and cutouts, *Door (inside)* (mirrored) for cable ducts and the door wiring; 3D: *Open door* / *Close door* |
| Wiring the panel | toolbar *Wiring*: pick a wire in the list or in 3D, *Wired* / *Problem*; web: open with `#wiring` |
| Project texts in several languages | *Project › Project languages…*: target languages, *Collect texts from the project*, translate (or CSV for translators); *File › Export as PDF in language* |
| Hoses / PCT loops | draw wires between fluid / P&ID ports; hose properties and configuration in the wire properties; *Project › Fluid and process…* (hoses, device groups, electro-fluid links and *Synchronize*, PCT loops) |
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
| `crates/openecraft-step` | STEP and IGES import: B-rep models tessellated into part meshes (uses `monstertruck`) |
| `crates/openecraft-jt` | JT writer and reader for triangle meshes (JT 8.1 layout, own implementation) |
| `crates/openecraft-client` | Requests to the parts and project servers and decoding of their answers, shared by the applications and the command line |
| `crates/openecraft-ui` | Shared egui library: part editor and previews, parts server client, file dialogs |
| `parts-server/` | `openecraft-parts-server`: parts database server (REST API, JSON storage) |
| `project-server/` | `openecraft-project-server`: shared projects with page locks for several users |
| `cli/` | `openecraft-cli`: command line and MCP server; one tool list (`src/tools/`) for both |
| `parts-manager/` | `openecraft-parts-manager`: standalone parts server maintenance (desktop + web) |
| `crates/openecraft-core/src/model.rs` | Project data model (pages, symbols, wires, parts) and page layout constants |
| `crates/openecraft-core/src/symbols.rs` | Built-in symbol libraries (IEC, NFPA, fluid power, P&ID, single-line, black boxes) |
| `crates/openecraft-core/src/symlib.rs` | User symbols: registry, editing helpers, library files |
| `crates/openecraft-core/src/forms.rs` | Title blocks (placeholders) and report forms |
| `crates/openecraft-core/src/production.rs` | Panel production: drilling pattern, door cutouts, NC export, wire end treatment, clearances |
| `crates/openecraft-core/src/harness.rs` | Wire harnesses: wires, connectors, bundle diameter, KBL export |
| `crates/openecraft-core/src/translation.rs` | Project texts in several languages: dictionary, translated output, CSV |
| `crates/openecraft-core/src/wiring.rs` | Wiring assistant: checklist order, wired / problem marks, progress, CSV |
| `crates/openecraft-core/src/fluid.rs` | Hoses and pipes between fluid ports: media, defaults, configuration, fluid netlist (port and line kinds), tube routing, tubing, checks |
| `crates/openecraft-core/src/electrofluid.rs` | Fluid device groups and the links between fluid and electrical devices: cross references, checks, synchronization |
| `crates/openecraft-core/src/process.rs` | Pre-planning: PCT loops, P&ID instruments, engineering status |
| `crates/openecraft-core/src/netlist.rs` | Connectivity analysis (nets / potentials, junctions) |
| `crates/openecraft-core/src/index.rs` | Device and cross-reference index |
| `crates/openecraft-core/src/reports.rs` | Reports and project check |
| `crates/openecraft-core/src/render.rs` | Backend independent page renderer (used by canvas and SVG) |
| `application/editor.rs` | Interactive canvas (tools, hit testing, pan/zoom) |
| `crates/openecraft-core/src/ops.rs` | Editing operations (placement defaults, move, rotate, paste, cable cores, page structure, …) |
| `crates/openecraft-core/src/structure.rs` | Structure identifiers: parse / complete / shorten tags (`=A1+CAB1-K1`) |
| `crates/openecraft-core/src/potentials.rs` | Potential definitions, presets, automatic wire colour and cross-section |
| `crates/openecraft-core/src/climate.rs` | Enclosure climate (IEC/TR 60890): power loss, surface, cooling and heating |
| `crates/openecraft-core/src/sizing.rs` | Electrical sizing: current-carrying capacity behind protective devices, voltage drop |
| `crates/openecraft-core/src/tables.rs` | Editing devices, connections and terminals in tables, CSV round trip |
| `crates/openecraft-core/src/config.rs` | Configurator: options, rules, page generation from macros |
| `crates/openecraft-core/src/wirenum.rs` | Wire numbers per connection and automatic numbering |
| `crates/openecraft-core/src/macros.rs` | Window / page macros, placeholders and value sets |
| `crates/openecraft-core/src/marking.rs` | Device, terminal, wire and cable markers, marking CSV |
| `crates/openecraft-core/src/strips.rs` | Terminal strips: order, renumbering, jumpers, spare terminals, part per terminal, strip layout with accessories, jumper parts |
| `crates/openecraft-core/src/plugs.rs` | Plugs: pin order, both sides of each pin, pin count of the parts, placement check |
| `crates/openecraft-core/src/plc.rs` | PLC points, addressing, symbol table export / import |
| `crates/openecraft-core/src/revision.rs` | Revisions: page snapshots, change detection, revision names, approval workflow |
| `crates/openecraft-core/src/compare.rs` | Baselines, project comparison and taking differences over |
| `crates/openecraft-core/src/review.rs` | Comments, redline / greenline markup, approvals, change requests (merged without locks) |
| `crates/openecraft-core/src/pdf.rs` / `dxf.rs` | PDF and DXF export (through the `render::Draw` trait) |
| `crates/openecraft-core/src/dxf_import.rs` | DXF import as a schematic page |
| `crates/openecraft-core/src/export3d.rs` | 3D scene of the panel layout, glTF, STEP, IGES, IFC and JT export |
| `crates/openecraft-parts/src/ifc.rs` | IFC import of part models (small STEP physical file reader) |
| `crates/openecraft-parts/src/bmecat.rs` | BMEcat catalog import with ETIM / eCl@ss features (small XML reader in `xml.rs`) |
| `application/app/engineering.rs` | Windows for macros, terminal strips and PLC overview |
| `application/app/revisions_ui.rs` | Revisions window with the approval workflow |
| `application/app/compare_ui.rs`, `application/app/review_ui.rs`, `application/app/requests_ui.rs` | Compare and baselines, review tab and tools, change requests |
| `application/app/symbol_editor.rs` / `application/app/forms_ui.rs` | Symbol editor and forms (title block / report form) windows |
| `application/app/harness_ui.rs` | Harness window |
| `application/app/languages_ui.rs` | Project languages window: dictionary, export per language |
| `application/app/process_ui.rs` | Hose properties and the fluid and process window (hoses, fluid netlist, device groups, electro-fluid links, PCT loops) |
| `application/wiring_ui.rs` | Wiring assistant workspace: checklist, current wire, 3D view, progress |
| `crates/openecraft-core/src/panel.rs` | Panel layout model, device sizes, rail snapping, collisions, duct routing |
| `crates/openecraft-core/src/panel/lineup.rs` | Line-ups: enclosures bayed side by side and the cable passages between them |
| `crates/openecraft-core/src/routing3d.rs` | 3D routing of wires, cables and tubes: rules, routing paths and reservations, grid search, bundles, shields, frozen routes |
| `crates/openecraft-core/src/collab.rs` | Shared projects: API types, lock targets and the merge rules of the project server |
| `application/panel_ui.rs` | Panel layout workspace: navigator, 2D editor, properties; `panel_ui/view3d.rs` the 3D view, `panel_ui/routing_ui.rs` the 3D routing (paths, reservations, rules, frozen routes) |
| `crates/openecraft-core/src/parts.rs` | Parts database, function templates, project parts, CSV import/export |
| `application/parts_ui.rs` | Parts management window, part selection dialog, parts server panel |
| `application/remote.rs`, `application/app/remote_ui.rs` | Project server client, window and shared editing (locks, merging); rules in `crates/openecraft-core/src/collab.rs` |
| `application/app.rs` | Application UI (menus, panels, dialogs); the windows in `application/app/*_ui.rs` (cables, tables, PLC, plugs, potentials, structure, pages, …) |
| `crates/openecraft-core/src/svg.rs` | SVG / print export |
| `application/io.rs` | File kinds of the editor (dialogs are in `openecraft-ui`) |
