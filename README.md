# OpenECraft

An open source electrical CAE editor written in Rust, in the spirit of EPLAN Electric P8.
One code base runs natively on **Windows, Linux and macOS** and in the **browser** (WebAssembly),
built with [egui / eframe](https://github.com/emilk/egui).

New to OpenECraft? [docs/tutorial.md](docs/tutorial.md) walks from the first page to the
bill of materials.

## Features

- **Languages**: English, German, French and Spanish (View › Language; the editor and the
  parts manager remember the choice and start in the system / browser language if supported).
  Project texts in scripts the bundled fonts lack (Chinese, Japanese, Korean) use the
  system's CJK fonts on the desktop as fallback, on screen and in PDF exports (embedded
  as a subset).
- **Projects with pages**: multi-line schematic pages (A3, 10 columns / 6 rows, title block),
  cover sheet and generated report pages. Page navigator with reordering.
- **IEC 60617 symbol library** (~330 symbols): relay coils (AC, latching, polarized,
  timers, time switches, counters, over- / undervoltage relays), NO/NC/changeover contacts
  (early / late, passing, on- / off-delay), push / pull / key / foot / selector switches
  with spring return or latching and position diagrams, emergency stops with turn / pull /
  key reset, cam / roller / pressure / flow / level / bimetal switches, inductive,
  capacitive, magnetic and photoelectric sensors, fuses (LV HBC), fuse switches and
  disconnectors, disconnectors and switch-disconnectors, circuit breakers, RCDs, motor
  protection switches, overload relays, AC / DC / EC motors and fans, signal lamps,
  beacons, sirens, indicating, recording and integrating meters, current transformers,
  1- and 3-phase transformers (vector groups) and autotransformers, rectifiers and
  converters, terminals, PLC inputs/outputs, potentials, interruption points, and
  electronics: resistors, capacitors (also polarized), diodes, Zener and photodiodes, LEDs,
  NPN / PNP transistors, N-channel MOSFET, thyristor, triac, NTC thermistor,
  photoresistor, optocoupler and bridge rectifier.
- **Installation plan symbols** (IEC 60617-11): 1-pole and two-way switches, dimmer, push
  button, lighting outlet, fluorescent luminaire, double socket outlet with PE, junction box,
  motion detector and distribution board, each with one connection point for the cable.
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
- **Numbering** (in the spirit of EPLAN's offline numbering): *Project › Device
  numbering…* renumbers device tags per structure and identifying letter in schematic
  order (page, column, top to bottom) after a format: `{letter}{n}` (`-K1`),
  `{letter}{page}.{n}` (`-K3.1`), `{letter}{page}{column}` (`-K34`) or NFPA style
  `{page}{column}{letter}` (`-34K`), with start value, step, letters and scope, and a
  preview of the new tags; everything that refers to a tag follows (parts, strips,
  plugs, PLC data, panel layout, harnesses). *Page › Renumber pages…* numbers pages
  with start and step, restarting per structure and keeping sub-pages (`3.a`; *Page › New
  sub-page* inserts `3.1`, `3.2`, … after the current page). The wire
  numbering gets a *Format* scheme with placeholders (`{page}`, `{column}`, `{row}`,
  `{potential}`, `{kind}`, `{color}`, `{cross_section}`, counter `{n}` counted per
  value of the rest), counter digits and step, and potential kinds that get no number
  (e.g. PE). Commands: `renumber_devices`, `renumber_pages`, `set_wire_numbering`.
- **Macros**: save a selection (window macro) or a whole page (page macro) into a macro
  library (stored with the application, JSON import / export). Window macros are placed
  with the mouse (ghost preview, devices get new tags), page macros insert a page.
  Placeholders such as `{MOTOR}` in tags, texts, function texts and designations are
  filled from **value sets** when inserting. **Variants**: a macro can hold the same
  function in several versions (A, B, … – e.g. direct on line and star-delta); save a
  selection under the name of an existing macro with another variant, choose the
  variant when inserting. **Macro boxes**: placed window macros remember their macro,
  variant, placeholder values and items (dashed frame on screen, *View › Macro boxes*);
  *Macros › Macro boxes › Update changed macros* replaces the content of boxes whose
  macro changed in the library, a box edited on the page is saved back into the
  library (command `list_macro_boxes`, tool `update_macro_boxes`).
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
  **Multi-level terminals**: a terminal on level 2 or higher sits on the block of the
  terminal before it (one part per block, tech data `Levels: 2`); the terminal
  diagram lists every level, the rail layout and the bill of materials count blocks,
  and the check reports more levels than the part has. **Jumper kinds**: plug-in
  jumpers (tech data `Jumper`), saddle jumpers (`Saddle jumper`) and wire jumpers (any
  level, no part); plug-in and saddle jumpers must stay on one level. Commands:
  `list_strips`, `get_strip`, `set_terminal`, `set_jumper_kind`, `set_strip`,
  `renumber_strip`.
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
  **Labeling schemes** choose the markers, the columns and their order, the separator,
  header and one row per printed label, for CSV / text and Excel export; built-in
  import tables for Phoenix Contact CLIP PROJECT, Weidmüller M-Print PRO, Brady,
  Murrplastik and Partex (starting points – copy and adjust to the program version),
  own schemes are stored in the project (commands `marking_schemes`,
  `set_marking_scheme`, exporters `marking` / `marking_xlsx` with `scheme`).
- **Strip and plug navigator** (navigator tab *Strips / plugs*): the terminal strips with
  their terminals in rail order (levels of multi-level blocks) and spares, the plugs with
  their pins and targets and the unused pins of the plug parts; a click shows a placed
  one, spares and unused pins are dragged into the page.
- **PLC navigator** (navigator tab *PLC*): the cards with their channels in address
  order, placed ones with function text and connected devices (a click shows the
  symbol), free channels of cards with a device description to drag into the page.
- **PLC overview**: all inputs / outputs per card with connected devices, editable
  addresses and function texts, card start byte / rack / slot, readdressing, **symbol
  table export / import** (CSV: name, data type, address, comment) for PLC programming
  tools, the tag table of **Siemens TIA Portal** (Excel sheet *PLC Tags*), the tag
  import of **Rockwell Studio 5000** (aliases of the module channels), the symbol table
  of **Siemens STEP 7 classic** (SDF), a global variable list for **Beckhoff TwinCAT 3**
  (TcGVL, `AT %I*` / `AT %Q*`), variables for **Schneider Control Expert** (XSY, `EBOOL`
  with topological addresses), device comments for **Mitsubishi GX Works** (CSV, `X` / `Y`
  in octal) and IEC 61131-3 global variables (CODESYS); addresses
  used on several cards are reported.
- **Revisions**: a revision stores a snapshot of all pages and can release (lock) them;
  locked pages are read only. Changes since the last released revision (added / changed /
  removed symbols, wires, texts, pages) are listed, marked on the pages with a revision
  triangle and reported in the *revision overview*; changed items name the changed
  properties with old and new value (`Device tag: -K1 › -K2; Position`).
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
  **PDF comments** a reviewer made in a PDF export (Acrobat or another viewer: notes,
  text boxes, highlights, shapes, replies) are imported into the review layer
  (*Review › Import PDF comments…*, command `import_pdf_comments`); rectangles and
  lines also become redline markup, comments imported before are skipped.
- **Change requests**: requests from inside or outside the project (number, title,
  description, requester, origin, priority, status, affected pages and devices,
  implementing revision) with linked comments (*Project › Change requests…*). Exchange
  with other systems as CSV (`Number;Title;Description;Requester;Origin;Date;Priority;
  Status;Pages;Devices;Revision`) or JSON; importing updates requests with a known number.
- **Symbol editor and libraries**: besides the IEC symbols there are NFPA / JIC
  symbols (30: coils, timer coils, contacts, timed and overload contacts, main
  contacts, push buttons, emergency stop, selector, foot, limit, float, pressure,
  temperature, flow and proximity switches, breakers, disconnects, fuses, pilot lights,
  horns, solenoids, control transformers, motors), fluid power symbols (ISO 1219: pumps, compressor, hydraulic motor, single and
  double acting cylinders, 3/2, 4/2 and 5/2 valves, non-return, relief, ball and flow
  control valves, pressure regulator, service unit, filter, accumulator, cooler,
  gauge, tank, silencer), single-line / overview symbols (cross references to the device, not part of
  the netlist) and black boxes / PLC boxes. National sets for China (GB), Russia and
  the CIS (GOST) and Japan (JIS) carry the device letters of GB/T 7159 (`KM`, `KA`,
  `QF`, `SB`, `HL` …), GOST 2.710 (`KM`, `KK`, `QF`, `SB`, `HL` …) and JIS C 0401
  (`MC`, `MCCB`, `PBS`, `PL`, `THR` …), so placed devices are numbered `-KM1`,
  `-MCCB1`; they are drawn like the IEC symbols, which GB/T 4728 and JIS C 0617 adopt
  (the ESKD proportions of the GOST drawings are not reproduced). Own symbols are drawn in the symbol editor
  (lines, rectangles, circles, texts, tag / label fields, connection points on the 4 mm
  grid, function such as main function, contact, terminal, PLC point), or copied from a
  built-in symbol; they are stored in the project and exchanged as library files.
- **Forms**: title blocks are lists of lines, rectangles and texts with placeholders
  (`{project}`, `{page}`, `{sheet}`, `{rev.date}`, user properties `{prop:Name}` …),
  edited in the title block editor and shared as files; one per project with an
  override per page (built-in: Standard, Compact, None). Report forms choose the
  columns, their order, the heading and the rows per report page.
  **Graphical forms**: a report form of the terminal, cable or plug diagram or the PLC
  assignment can switch to its graphical form – the strip drawn with a box per terminal (levels marked), end
  brackets and end plates, jumper bridges and the external and internal targets with
  potential, function text and linked location; or each cable as a band with its cores
  from end to end (potential, wire number, reserve cores dashed); the plug diagram as
  a housing with socket and pin half per pin; the PLC assignment as a module per card
  with the inputs coming in from the left and the outputs going out to the right.
  **Drawn forms**: any report (also those of plugins) can be drawn with an own form –
  graphics drawn once per page and a row drawing repeated for every row, downwards for
  lists or sideways (e.g. one terminal beside the other); its texts take the columns as
  placeholders (`{Device tag}`, `{Part number}` …), `{report}` and the title block
  placeholders. The form editor (*Report forms › Own drawn form*) starts from the
  report's table and shows the page with its real rows while lines, rectangles and
  texts are added and dragged.
- **Live cross references**: contacts show where their coil is (`/3.4`), coils show a
  contact image with all contacts and their locations, interruption points reference their
  counterparts on other pages. *Project › Cross references…* sets the format (prefix,
  separator, row: `/3.4`, `3-4`, `/3.4.B`), when the structure of the target page is
  shown (when it differs, always, never), what interruption points refer to (all
  other points, star from the first point, or pairs in schematic order; the check
  reports a point without partner) and whether coils draw a contact image. Command:
  `set_cross_references`.
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
  **Message management** (in the spirit of EPLAN's check schemes): every message has
  a code per kind (`wires.3f9a1c`, the same in every language) and a fingerprint per
  occurrence. Right click a message in the Check tab to accept it as checked (hidden
  and not counted until it changes), to change the level of its kind (error, warning,
  note, off) or to switch its check off; the settings (⚙) list the checks, changed
  levels and accepted messages (with note, user, date and whether they are still
  reported). The tab filters by level, page and text and exports the messages as CSV.
  The settings are stored with the project; the checks switched off and the changed
  levels are exchanged as a **check scheme** file (JSON) between projects. Commands:
  `list_checks`, `check_settings`, `set_check`, `set_message_level`, `accept_message`;
  tools `export_check_scheme`, `import_check_scheme`; `check` shows codes and
  fingerprints.
- **Parts management** (EPLAN style):
  - a global **parts database** shared by all projects (product top group / group /
    subgroup tree, filter), with general, commercial (price, price unit, packaging,
    quantity unit), technical, mounting (dimensions, weight) data, accessories and a
    **function template** per part (e.g. coil A1/A2, main contacts, 13/14 NO);
  - **assemblies**: a part can consist of component parts with quantities (tab
    *Accessories*, CSV column `Components`: `NUM:2|NUM2`); the bill of materials lists
    them under the assembly (nested assemblies resolved, priced as included);
  - **typed mounting data** per part (tab *Mounting*): clearances (top, bottom, left,
    right, front, EMC, service space), mounting holes with thread, door and face
    cutouts, power loss, how far an accessory sticks out, fastening, faces, collision
    class and terminal type. They drive the panel checks, drilling, cutouts and the
    climate calculation; CSV has a column per value (`Power loss`, `Mounting holes`,
    …). Older parts with these values as `Key: value` lines in the technical data
    still work; imports and the built-in catalog move such lines into the typed
    fields, the part editor offers it for existing parts;
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
  - **EPLAN parts XML**: the parts management export of EPLAN (`<part>` with
    `P_ARTICLE_*` attributes and variants, multi-language texts, product groups by
    their exchange numbers) is imported like the other formats, and parts export as
    EPLAN parts XML for the way back (parts management, `parts_export --format
    eplan_xml`);
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
    hinge; the mounting layout page shows the plate and every face with items (door,
    side walls, rear wall, roof, bottom) side by side;
  - copper **busbar systems** (bars drawn, metres and holders in the BOM) and rectangular
    or round **cutouts** in the mounting plate;
  - **drilling pattern**: rails, ducts and busbar holders get tapped holes along their
    length, devices screwed onto the plate get holes from their part data
    (`Mounting holes: M5: 9,12; 69,72`) or two default holes; **NC export** per
    enclosure and face as DXF or DWG (outline, one layer per hole diameter, cutouts)
    or CSV;
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
  - **room**: the 3D view shows all enclosures side by side at their places, each
    line-up at its position and turn set by hand or taken from the plant layout;
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
  drops above 5 %. The capacity is corrected for ambient temperature, bunched
  circuits (IEC 60364-5-52, B.52.17), aluminium and XLPE insulation.
- **Electrical calculations and verification** (*Project › Electrical calculations…*):
  - *Supply*: voltage, earthing system (TN-S, TN-C, TN-C-S, TT, IT), highest and lowest
    short-circuit current at the supply point, rated current;
  - *Short circuit and protection coordination* (IEC 60909-0 simplified, IEC 60364-4-41,
    -4-43): the protective devices of the mains with their time-current bands
    (IEC 60898-1 B / C / D / K / Z, motor protection class 10, gG / aM fuses; part tech data
    `Characteristic`, `Breaking capacity`, `Instantaneous trip`, `Let-through energy`),
    breaking capacity against the short-circuit current, disconnection within 0.4 s /
    5 s (TT: 0.2 s / 1 s, residual current device needed) at the lowest fault current at
    the end of each circuit, k²S² withstand of the cables, selectivity with the device
    upstream (total / partial up to a current; gG fuses at a ratio of 1.6); reports
    *Short circuit and disconnection*, *Selectivity*;
  - *Load calculation*: motors, drives, heaters, power supplies and transformers of the
    mains with active / apparent power and current (`Power`, `Current`, `Power factor`,
    `Efficiency`), the load of each protective device, demand with the rated diversity
    factor of IEC 61439-1 (table 101) or a set factor, phase currents and unbalance, and
    the load budget of each 24 V power supply (`Coil power`, `Current consumption`);
    reports *Load calculation*, *Power supply load budget*;
  - *EMC*: motor cables of frequency converters shielded at both ends, analogue signals
    in shielded cables, no power and signal conductors in one cable, surge protection;
  - *Functional safety* (ISO 13849-1): safety functions of subsystems with their devices,
    category, DCavg, operations per year and CCF; PL by the simplified procedure
    (figure 5) from `MTTFd` / `B10d` of the parts or certified `PL` / `PFHd`, combined by
    table 11, with the matching SIL; report *Safety functions*;
  - *Rule sets*: built-in libraries (IEC 60204-1, IEC 61439-1, UL 508A / NFPA 79,
    ISO 13849-1, EMC) and customer rule sets (JSON import / export) that tighten the
    limits (voltage drop, disconnection times, minimum cross-sections, load reserve,
    phase unbalance, total selectivity, shielding, surge protection) and add checks
    "every item of a scope (project, pages, devices, parts, cables, connections,
    potentials) that meets these conditions must meet this one"; report *Rule set
    verification*; all results are part of the project check.
- **Editing in tables** (*Project › Edit in table…*): devices (tag, function text,
  description, part), connections (wire number, colour, cross-section, cable, core) and
  terminals (strip, number, function text) as rows; filter, edit cells in place, fill a
  column or find and replace in the filtered rows; CSV export / import for changes in a
  spreadsheet (rows by key, only changed cells are applied); rows on released pages
  stay unchanged. Renaming a device moves its strip, plug and PLC card data along.
  The round trip also works with **Excel** files (`.xlsx`: export, change, import);
  report tables export as Excel workbooks from the Reports window and with `report
  --format xlsx --path …` (bold, frozen header row with filter, numbers as numbers,
  codes such as `007` kept as text).
- **Editing**: select / window select / move with rubber-band wires, rotate, mirror,
  copy / cut / paste (via clipboard, devices get new tags), undo / redo, texts, grid
  snapping, pan and zoom. Editing aids: grid sizes 1, 2, 4, 8 and 16 mm (View); a
  coordinate field in the tool bar (`x y` absolute or `@dx dy` relative to the last
  point, Enter acts like a click of the current tool); *Edit › Align* (edges or
  centres), *Distribute* (evenly, horizontally or vertically), *Move selection by…*
  and *Multiple copy…* (n copies at a fixed distance, new tags and terminal numbers);
  attached wire ends follow. Commands: `align_items`, `distribute_items`, `copy_items`.
- **Texts**: free texts with rotation (0 … 359°, e.g. reading upwards along a terminal
  strip), left / centred / right alignment, a vertical anchor, word wrap at a set
  width, bold / italic / underline, a border and an opaque background, and a hyperlink
  (Ctrl + click opens it; clickable in the PDF and SVG export). **Special texts** show
  project and page properties anywhere on a page: placeholders such as `{page}`,
  `{project}` or `{prop:Name}` (the same as in title blocks, Insert › Special text) are
  filled in when the page is drawn. A **path function text** (Insert › Path function
  text) is the function text of all functions in its column that have none of their
  own; the device list, terminal, plug and PLC diagrams use it. Texts turn with a
  rotated selection; DXF / DWG import keeps text rotation and alignment. Command line /
  MCP: `add_text`, `update_text`.
- **Schematic graphics**: lines, rectangles, circles, ellipses, arcs, polylines and
  polygons (width, colour, dashed, filled), dimensions (aligned / horizontal / vertical,
  scale, unit), PNG / JPEG images (also in the PDF and SVG export). **Layers**
  (navigator › Layers): visible, locked, printed, colour; DXF / DWG export per layer.
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
- **Find** (*Edit › Find…*, `Ctrl+F`): searches the whole project (page names and
  descriptions, device tags, function texts, free texts, wire numbers, terminals and
  pins, cables, part numbers, descriptions) with case and whole-word options, jumps
  to the hits and replaces text in free texts, function texts and descriptions
  (released pages stay). Commands: `find`, `replace_text`.
- **Navigators**: pages (optionally grouped by structure with the descriptions of the
  structure management), devices (select on page, place further functions, rename),
  connections (filter, show, select, add a connection definition), cables (cores in
  use and free, cable definitions, draw a cable definition), layers and the check.
  The *Unplaced* tab lists what the project data has but the schematic does not: the
  channels of PLC cards with a device description that have no input / output symbol
  (with their address), and the functions of the assigned parts that no symbol uses
  (coils, contacts with their designations), and the potentials defined in the project
  that no potential connection point carries yet; drag one into the page (or click it, then
  into the page) to place it with tag, address and designations. Command:
  `list_unplaced`.
- **Output / data exchange**: **PDF** of all pages or a page selection (`1-3, 5`;
  also for the HTML print document; *File › Print* opens all or the current page in the
  browser with its print dialog) with bookmarks per page, clickable cross
  references and report rows (links to pages left out are dropped); texts beyond Latin-1 such as Polish, Czech, Turkish,
  Greek or Cyrillic are written with an embedded subset of the Ubuntu font, searchable
  and copyable, **DXF** (R12, layers by kind) and **DWG** (AutoCAD 2004,
  the same drawing) per page or all pages side by side, SVG per page, printable HTML, production wire list (CSV with wire numbers,
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
  **DXF / DWG import** (*File › Import DXF / DWG as page…*): old drawings as graphics
  on a new schematic page – lines, polylines, circles, arcs, ellipses, splines, texts,
  MTEXT, block references and dimensions, in their units, scaled down to the drawing
  area when they are larger. DWG files of AutoCAD R12 … 2018 are read (and written)
  with the pure Rust [acadrust](https://crates.io/crates/acadrust) crate, also in the
  browser. EPLAN's EDZ format is proprietary and not supported.
- **Wire harnesses**: single point-to-point connections (`-S1:13 – -K1:A1`) are
  assigned to harnesses (`-WH1`), independent of the other connections of the same
  potential section; assignments follow renamed devices, missing connections are
  reported by the check; the *harness data*
  report lists their wires with length (routed in the panel layout or the harness's
  field length) and end treatment, the harness window shows the connectors with their
  pin assignment and the bundle diameter; export as simplified **KBL** (VDA 4964, XML)
  for harness manufacturing. **Nailboard drawings** (formboards) to scale: the bundle as
  branches from a common node to the connectors, each as long as the wire lengths
  require and as thick as its bundle, with nails, branch lengths and the pin table of
  every connector, on A3 at the smallest standard scale that fits (harness window
  › *Nailboard (SVG)…*, exporter `nailboard`, command `harness_nailboard`).
- **Wiring assistant** (in the spirit of EPLAN Smart Wiring): the *Wiring* workspace
  turns the routed wires of the panel layout into a touch-friendly checklist for the
  workshop – ordered by device, position, cross-section or wire number – with the
  current wire highlighted in 3D, its colour, cross-section, length, stripping length
  and ferrule, and the progress in wires and metres. Wires are marked as wired or as a
  problem (with a note); marks are stored in the project, so wires changed in the
  schematic after they were wired are flagged by the check. The web build opens the
  assistant directly with `…/#wiring` (desktop: `--wiring`), which makes a tablet in
  the workshop a cheap wiring station; checklist with state as CSV.
- **Mounting assistant** (in the spirit of EPLAN Smart Mounting): the *Mounting* switch
  of the *Wiring* workspace lists the items of the panel layout in mounting order –
  cutouts, rails, ducts and busbars, devices, accessories, then door and walls – with
  the current item highlighted in 3D, its place, size and part, and the progress.
  Items are marked as mounted or as a problem (with a note); items moved or given
  another part after they were mounted are flagged by the check. Commands
  `mounting_list` and `mark_mounted`; checklist with state as CSV.
- **Enterprise integration** (*Project › Enterprise integration…*): one integration
  layer with connectors for **ERP** (item master and BOM, CSV), **PDM** (project and
  page metadata, JSON), **PLM** (engineering BOM, JSON), **ECAD** (components and
  netlist for PCB design, JSON), **MCAD** (device placement, CSV), **procurement**
  (order list per supplier, stock and pack sizes considered, CSV) and **wiring
  machines** (job list with cut length, stripping and ferrules, CSV). All connectors
  work in both directions and share one neutral field vocabulary: exports use the
  field codes as columns / keys, imports recognise the codes, common English and
  German column names (`Material`, `Bestand`, `refdes` …) and the column mapping of a
  *profile* (e.g. `MATNR` → `erp_number` for SAP). Imports are shown as a list of
  changes that are reviewed and taken over: ERP numbers, prices, stock and lead
  times, PLM item IDs, revisions and lifecycle states (obsolete parts in use are
  flagged by the check), PDM document numbers and states, part assignments from the
  ECAD system, device positions from the MCAD system, order confirmations, and the
  wire processing feedback as wiring marks. Each export stores the exported values,
  so values changed on both sides since are reported as conflicts and values the
  other system left unchanged never overwrite local edits. A neutral exchange file
  (`openecraft.exchange/1`, JSON) holds everything for systems without a connector;
  the exchange log is kept in the project.
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
- **Pre-planning** (*Project › Pre-planning…*): the plant as a tree of segments
  (plant, area, unit, …) with function and location inherited down the tree and the
  devices planned in them (tag, kind, description, power, voltage, part, page macro);
  the planned power per segment, segment templates, the state of every planned device
  against the schematic (planned, engineered, part differs; also a check and the
  report *Pre-planning*), schematic pages generated from the page macros of the
  devices not engineered yet, and the exchange as CSV / Excel (commands
  `preplanning`, `add_segment`, `plan_device`, …; tools `import_preplanning`,
  `generate_preplanning_pages`).
- **Single-line / multi-line engineering** (*Project › Single-line / multi-line…*):
  devices are planned with single-line symbols (breaker, switch-disconnector, motor
  protection switch, fuse, overload relay, contactor, motor, transformer) on overview
  pages and engineered in the multi-line schematic; both share the device tag. The
  window lists each device as planned, engineered, not in the single-line diagram or
  not matching, and generates the schematic of a single-line page (3-pole power
  symbols, conductors, contactor coils) or a single-line diagram of the schematic's
  power devices (arranged from the supply to the loads). The check reports planned
  devices, mismatching symbols, connections of the single-line diagram missing in the
  schematic and differing function texts; *Synchronize function texts* fills empty
  ones from the other representation.
- **Project texts in several languages**: function texts, page descriptions, free
  texts and descriptions are written in a source language; the project's translation
  dictionary (*Project › Project languages…*) translates them line by line into any
  number of target languages, with CSV export / import for translators and a check
  for untranslated texts. *File › Export as PDF in language* (command line: `export`
  with `language`) puts out the documentation per language – project texts from the
  dictionary, reports and title blocks in that language (English, German, French,
  Spanish; other languages get English report headings).
- **Project templates** (in the spirit of EPLAN's basic projects): *File › New project
  from template* starts from a built-in template (empty IEC 60204-1 project; machine
  with cover sheet, table of contents, structured schematic pages and report pages;
  distribution board with single-line overview, climate and short-circuit reports) or
  from any project file; *Save as project template…* writes the current project
  without revisions, baselines, review, wiring marks and accepted check messages.
  Command line: `new_project --template Machine` (or a file).
- **Storage**: JSON project files (`.oec`); the current project and the parts
  database are also auto-saved (compressed) in the app's data folder on the desktop and
  in IndexedDB in the browser, whose local storage holds only about 5 MB (until the
  first write to IndexedDB succeeds they are kept in local storage as well).

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

### Parts server (data portal)

`openecraft-parts-server` is the data portal of all OpenECraft installations (desktop
and web), in the spirit of the EPLAN Data Portal: manufacturer catalogs with a faceted
search, part history and lifecycle, files of the parts (datasheets, images, models,
macros), user accounts with roles and a web page for browsing. It is a separate
executable and runs independently of the editor.

```bash
cargo run --release -p openecraft-parts-server -- --bind 0.0.0.0:8090 --token <secret>
cargo run --release -p openecraft-parts-server -- --storage json --data parts-db.json
cargo run --release -p openecraft-parts-server -- --data parts-db.sqlite --migrate-from parts-db.json
cargo run --release -p openecraft-parts-server -- --install-catalogs all
```

- Storage (`--storage sqlite|json`): SQLite by default (`--data parts-db.sqlite`, one
  file with indexed parts, history, accounts and files); JSON keeps everything in one
  file written atomically on every change, with the files in `<name>.files/` next to
  it. A `.json` data file selects JSON storage. New databases are filled with the
  example catalog and the bundled manufacturer catalogs; `--migrate-from` copies an
  existing database into a new one.
- Bundled catalogs: `parts-server/catalogs/<id>.json` (a `catalog` and its `parts`,
  listed in `parts-server/src/bundled.rs`) are curated starter sets of real
  manufacturer parts: `abb` (circuit breakers, RCCB, contactors, manual motor
  starters), `beckhoff` (EtherCAT coupler and terminals), `dehn` (surge protective devices type 1 - 3 and for data lines), `hager` (circuit breakers,
  RCCBs, installation contactors, switch disconnectors), `mdt` (KNX actuators, IP
  interface / router, power supply), `phoenix_contact` (terminal blocks, relay
  modules, power supplies), `siemens` (SIRIUS contactors, overload relays, motor
  starter protectors, SENTRON circuit breakers) and `weidmueller` (terminals, relay
  module, power supplies, CD wiring ducts). `--install-catalogs all` (or ids such as
  `siemens`) installs or updates them in an existing database: new parts are added,
  parts nobody changed are updated, parts changed or deleted by users are kept. Parts
  without models get generated ones from their mounting dimensions. Only add data
  that may be redistributed (facts from public data sheets, no copied texts, images
  or models).
- Users: readers keep favorites, manufacturers change the parts and catalogs of their
  own manufacturer, editors all parts, administrators also manage the users. The
  `--token` (or `OPENECRAFT_TOKEN`) is the administrator token; `--admin-user <name>`
  creates an administrator account with the password from `OPENECRAFT_ADMIN_PASSWORD`.
  Users sign in with `POST /login` and send the session token as
  `Authorization: Bearer <token>`. Without token and accounts the server is read-only.
- Every change is a new revision; each part keeps its revision, time of change and the
  last 20 versions (history). Parts have a lifecycle (active, phase-out, discontinued)
  and a successor. Files are stored once per content; a file is deleted as soon as
  neither a part nor a version in the history uses it (and at startup).
- Product groups: the EPLAN generic product groups, product groups and subgroups with
  their exchange numbers (`openecraft_parts::productgroups`) are offered in the part
  editor next to OpenECraft's own groups.
- Search (`POST /search`): text (every word must occur), top group, group, subgroup,
  manufacturer, catalog, lifecycle and technical data (voltage, current, power and the
  `Name: value` lines of the technical data), sorted and paged, with facets that count
  the matching parts per value.
- Web page: `http://<server>:8090/` – search with facets, part details with files and
  history, catalogs, sign-in and favorites, in English, German, French and Spanish.
- REST API (`/api/v1`, see `openecraft_parts::api`): `GET /info`, `GET /parts`,
  `POST /search`, `GET|PUT|DELETE /parts/{number}`, `GET /parts/{number}/history[/{revision}]`,
  `POST|DELETE /parts/{number}/attachments`, `GET /files/{id}`, `POST /import` (CSV,
  JSON or BMEcat), `GET /export.csv`, `GET|PUT|DELETE /catalogs/{id}`, `POST /login`,
  `POST /logout`, `GET|PUT /favorites`, `GET|PUT|DELETE /users/{name}`.
- In the editor: *Project › Data portal…* – search the server, add parts to the local
  database, save their files, add their macros to the macro library and check the
  database for newer versions, phased out and discontinued parts. *Project › Parts
  management › 🌐 Parts server* downloads all parts or uploads parts. The project check
  warns about devices with discontinued parts; `use_successor_parts` changes them over.
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
  `Authorization: Bearer <token>`; without it and without user accounts everyone who
  reaches the server can change its projects.
- **User accounts** (`users.json` in the data directory, Argon2 password hashes):
  viewers open projects read-only (comments and markup), editors lock, change, save
  and create projects, administrators also manage the users; a user can be limited to
  some projects (names with `*` / `?`). `--admin-user <name>` creates the first
  administrator with the password from `OPENECRAFT_PROJECT_ADMIN_PASSWORD`. Users sign
  in from *File › Project server…* (user name, password, *Sign in*); the server uses
  the account's name on the locks, whatever the request says.
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
  `POST /projects/{name}/open|status|lock|release|close`, `POST /login`, `POST /logout`,
  `GET /users`, `PUT|DELETE /users/{name}` (see `openecraft_core::collab`).

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

Enter the server URL and the access token (kept in memory only) and click *Connect*, or
sign in with a user name and password. Changes are saved to the server with *Save*
(`Ctrl+S`); the editor asks before unsaved changes are discarded and before a part is
deleted. *Files and history* above the part editor uploads and removes the files of a
part and shows its changes; *Catalogs* maintains the manufacturer catalogs and *Users*
(administrators) the accounts and roles.

### Command line and MCP server (AI agents)

`openecraft-cli` edits projects and maintains the parts server without a window. The same
commands are offered to AI agents over the [Model Context Protocol](https://modelcontextprotocol.io)
(`openecraft-cli mcp`, JSON-RPC on stdin / stdout), so an agent can create projects, place
symbols, draw wires between connection points, assign parts, run the check, generate
reports and export PDF / SVG / DXF / DWG.

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
  environment variables or the `parts_connect` tool (also with `user` / `password`);
  default `http://127.0.0.1:8090`. The data portal tools search with facets
  (`parts_search`), manage catalogs, files, favorites and users (`parts_catalog_save`,
  `parts_attach`, `parts_favorites`, `parts_user_save`, …) and compare the project parts
  with the server (`parts_check_updates`).
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

### Plugins

Plugins add commands, checks, reports and exports to the editor (*Plugins* menu, *Check*
tab), the command line and the MCP server at the same time. Built-in plugins are Rust
code in `crates/openecraft-api`; external plugins are programs in any language that
answer JSON requests on stdin / stdout:

```bash
OPENECRAFT_PLUGINS=examples/plugins openecraft-cli -p pump.oec plugins
openecraft-cli --plugins examples/plugins -p pump.oec report --kind page_notes
```

See [docs/plugins.md](docs/plugins.md) for the API and the protocol.

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
| Settings scheme | *Project › Settings scheme*: export the project's wire numbering, potentials, wire defaults and colours, marking, hose defaults, cross references and check scheme as one file and take them into other projects (tools `export_settings`, `import_settings`) |
| Docking panels | Drag the tab of a navigator, the symbols, the properties or the page to dock it beside, above or below another one or as a tab among others; *View › Panels* shows or hides each and resets the layout (kept by the app) |
| Workspaces | *View › Workspaces*: save the workspace, the arrangement of the panels and the open windows under a name and restore them with one click (kept by the app) |
| Keyboard shortcuts | *View › Keyboard shortcuts…*: record, clear or reset the keys of open, save, undo / redo, find, select all, page up / down, fit, wire and text tool, rotate, mirror, wire corner, routing and finishing polylines; conflicts are shown, the keys are kept by the app and shown in the menus and the help |
| Change requests | *Project › Change requests…*: requests, CSV / JSON import and export |
| Export | *File*: PDF, DXF / DWG (page / all), SVG, HTML, production wire list, panel layout in 3D (glTF / STEP / IGES / IFC / JT) |
| Import old drawings | *File › Import DXF / DWG as page…* |
| Own symbols | *Project › Symbol editor…*; they appear under *User symbols* in the symbol list |
| Title blocks / report forms | *Project › Title blocks and report forms…*; per page in the page properties |
| Panel production | panel layout: *Busbar* / *Cutout* tools; enclosure properties: NC data (DXF / DWG / CSV) |
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
| `crates/openecraft-parts` | Shared library: part model, function templates, CSV/JSON, catalog, EPLAN product groups, server API types, data portal (faceted search, lifecycle, update check), 3D math |
| `crates/openecraft-step` | STEP and IGES import: B-rep models tessellated into part meshes (uses `monstertruck`) |
| `crates/openecraft-jt` | JT writer and reader for triangle meshes (JT 8.1 layout, own implementation) |
| `crates/openecraft-client` | Requests to the parts and project servers and decoding of their answers, shared by the applications and the command line |
| `crates/openecraft-ui` | Shared egui library: part editor and previews, parts server client, data portal browser, file dialogs |
| `crates/openecraft-api` | Extension API: plugins with commands, checks, reports and exports for all front ends; external plugins over JSON |
| `parts-server/` | `openecraft-parts-server`: data portal server (REST API, SQLite or JSON storage, users, web page) |
| `project-server/` | `openecraft-project-server`: shared projects with page locks for several users |
| `cli/` | `openecraft-cli`: command line and MCP server; one tool list (`src/tools/`) for both |
| `parts-manager/` | `openecraft-parts-manager`: standalone parts server maintenance (desktop + web) |
| `crates/openecraft-core/src/model.rs` | Project data model (pages, symbols, wires, parts) and page layout constants |
| `crates/openecraft-core/src/symbols.rs` | Built-in symbol libraries (IEC, NFPA, GB / GOST / JIS, fluid power, P&ID, single-line, black boxes) |
| `crates/openecraft-core/src/symlib.rs` | User symbols: registry, editing helpers, library files |
| `crates/openecraft-core/src/forms.rs` | Title blocks (placeholders) and report forms |
| `crates/openecraft-core/src/production.rs` | Panel production: drilling pattern, door cutouts, NC export, wire end treatment, clearances |
| `crates/openecraft-core/src/harness.rs` | Wire harnesses: wires, connectors, bundle diameter, KBL export |
| `crates/openecraft-core/src/translation.rs` | Project texts in several languages: dictionary, translated output, CSV |
| `crates/openecraft-core/src/wiring.rs` | Wiring assistant: checklist order, wired / problem marks, progress, CSV |
| `crates/openecraft-core/src/mounting.rs` | Mounting assistant: mounting order, mounted / problem marks, progress, CSV |
| `crates/openecraft-core/src/integration.rs` | Integration layer: neutral fields, connectors (ERP, PDM, PLM, ECAD, MCAD, procurement, wiring machine), profiles, import preview, conflicts, exchange log |
| `crates/openecraft-core/src/fluid.rs` | Hoses and pipes between fluid ports: media, defaults, configuration, fluid netlist (port and line kinds), tube routing, tubing, checks |
| `crates/openecraft-core/src/electrofluid.rs` | Fluid device groups and the links between fluid and electrical devices: cross references, checks, synchronization |
| `crates/openecraft-core/src/process.rs` | Pre-planning: PCT loops, P&ID instruments, engineering status |
| `crates/openecraft-core/src/preplanning.rs` | Pre-planning: segments, planned devices, templates, power budget, page generation, CSV / Excel |
| `crates/openecraft-core/src/singleline.rs` | Single-line / multi-line engineering: device status, checks, transformation in both directions |
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
| `crates/openecraft-core/src/electrical.rs` | Supply, installation conditions, circuits of the mains |
| `crates/openecraft-core/src/protection.rs` | Protective devices: characteristics, time-current bands, let-through energy |
| `crates/openecraft-core/src/shortcircuit.rs` | Short-circuit currents, disconnection, withstand, selectivity |
| `crates/openecraft-core/src/loads.rs` | Load calculation, diversity, phase balance, power supply budgets |
| `crates/openecraft-core/src/emc.rs` | EMC checks of the cables |
| `crates/openecraft-core/src/safety.rs` | Safety functions and performance level (ISO 13849-1) |
| `crates/openecraft-core/src/rules.rs` | Rule libraries, customer rule sets, limits and verification |
| `crates/openecraft-core/src/tables.rs` | Editing devices, connections and terminals in tables, CSV round trip |
| `crates/openecraft-core/src/config.rs` | Configurator: options, rules, page generation from macros |
| `crates/openecraft-core/src/wirenum.rs` | Wire numbers per connection and automatic numbering |
| `crates/openecraft-core/src/macros.rs` | Window / page macros, placeholders and value sets |
| `crates/openecraft-core/src/pdf_comments.rs` | Import of PDF comments (annotations) into the review layer; minimal PDF object reader |
| `crates/openecraft-core/src/marking.rs` | Device, terminal, wire and cable markers, labeling schemes, marking CSV / Excel |
| `crates/openecraft-core/src/strips.rs` | Terminal strips: order, renumbering, jumpers, spare terminals, part per terminal, strip layout with accessories, jumper parts |
| `crates/openecraft-core/src/plugs.rs` | Plugs: pin order, both sides of each pin, pin count of the parts, placement check |
| `crates/openecraft-core/src/plc.rs` | PLC points, addressing, symbol table export / import |
| `crates/openecraft-core/src/revision.rs` | Revisions: page snapshots, change detection, revision names, approval workflow |
| `crates/openecraft-core/src/compare.rs` | Baselines, project comparison and taking differences over |
| `crates/openecraft-core/src/review.rs` | Comments, redline / greenline markup, approvals, change requests (merged without locks) |
| `crates/openecraft-core/src/pdf.rs` / `dxf.rs` | PDF and DXF export (through the `render::Draw` trait) |
| `crates/openecraft-core/src/dxf_import.rs` | DXF import as a schematic page |
| `crates/openecraft-core/src/dwg.rs` | DWG export and import (converted from / to DXF with `acadrust`) |
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
| `application/app/singleline_ui.rs` | Single-line / multi-line window: device status and the transformations |
| `application/app/integration_ui.rs` | Enterprise integration window: export, import preview, profiles, linked data, log |
| `application/wiring_ui.rs` | Wiring assistant workspace: checklist, current wire, 3D view, progress |
| `application/mounting_ui.rs` | Mounting assistant: checklist in mounting order, current item, 3D view, progress |
| `crates/openecraft-core/src/panel.rs` | Panel layout model, device sizes, rail snapping, collisions, duct routing |
| `crates/openecraft-core/src/panel/lineup.rs` | Line-ups: enclosures bayed side by side and the cable passages between them |
| `crates/openecraft-core/src/routing3d.rs` | 3D routing of wires, cables and tubes: rules, routing paths and reservations, grid search, bundles, shields, frozen routes |
| `crates/openecraft-core/src/collab.rs` | Shared projects: API types, lock targets and the merge rules of the project server |
| `application/panel_ui.rs` | Panel layout workspace: navigator, 2D editor, properties; `panel_ui/view3d.rs` the 3D view, `panel_ui/routing_ui.rs` the 3D routing (paths, reservations, rules, frozen routes) |
| `crates/openecraft-core/src/parts.rs` | Parts database, function templates, project parts, CSV import/export |
| `application/parts_ui.rs` | Parts management window, part selection dialog, parts server panel, data portal window |
| `application/remote.rs`, `application/app/remote_ui.rs` | Project server client, window and shared editing (locks, merging); rules in `crates/openecraft-core/src/collab.rs` |
| `application/app.rs` | Application UI (menus, panels, dialogs); the windows in `application/app/*_ui.rs` (cables, tables, PLC, plugs, potentials, structure, pages, …) |
| `crates/openecraft-core/src/svg.rs` | SVG / print export |
| `application/io.rs` | File kinds of the editor (dialogs are in `openecraft-ui`) |
