# Getting started: from the first page to the bill of materials

This tutorial walks through a small control circuit in OpenECraft: pages, symbols,
wires, devices with parts, terminals, a PLC, the project check, reports, the panel
layout and the output. Menu paths are written as *Menu › Entry*; the user interface
is available in English, German, French and Spanish (*View › Language*), the
names below are the English ones.

The fastest way to see everything at once is *File › Open demo project*: a conveyor
control with power and control circuits, terminals, a cable, a PLC, a harness and a
panel layout. Most steps below can be tried on it.

## 1. A new project

- *File › New project* starts an empty project with one schematic page.
  *File › New project from template* starts from a built-in template (IEC or NFPA
  style frames, structure, potentials) or *From a template file…* from any saved
  project.
- *Project › Project properties…* holds the project data shown in the title blocks
  (name, customer, creator) and the default plot frame and title block.
- The project is saved as one `.oec` file (*File › Save*, Ctrl+S). In the browser the
  project is kept in the browser storage and downloaded with *File › Download project*.

## 2. Pages and structure

- The *Pages* tab of the navigator on the left lists the pages; *Page › New schematic
  page* adds one after the current page. Other page kinds: cover sheet, overview,
  graphics, document (an embedded PDF or image), report and mounting layout pages.
- *Page › Page properties…* gives a page its description and its structure: the
  function (`=A1`) and location (`+CAB1`). Devices placed on the page take this
  structure (`=A1+CAB1-K1`); on the page only the part that differs is shown.
- The 🗄 button of the *Pages* tab groups the pages by structure; *Page › Renumber
  pages…* renumbers them. *Page › New sub-page* inserts `3.1`, `3.2`, … after the
  current page; renumbering keeps them sub-pages of their page.

## 3. Placing symbols

- The symbol list is on the right (*Symbols*), grouped by library (IEC 60617,
  NFPA, fluid power, P&ID, installation plan, …) with a search field. Click a symbol,
  then click into the page to place it. Placing continues until Esc or a right click.
- R rotates and M mirrors the symbol being placed (or the selection). Keys can be
  changed in *View › Keyboard shortcuts…*.
- Device tags are assigned automatically from the identifying letter (`-K1`, `-S2`).
  To place contacts of a contactor, select its coil first: contacts placed then
  belong to it and get the next free numbers (13/14, 21/22, …). Contacts and coils
  show their cross references (`/3.4`) live.
- Select a symbol to edit it in the property panel on the right: tag, function text,
  connection point designations, the parts of the device and further properties.

## 4. Wires and potentials

- W starts the wire tool: click the start, intermediate corners and the end; Space
  switches the corner direction, Enter or a double click finishes. With
  *Autoconnect* on, symbols placed in line with others are connected automatically.
- Potentials (`L1`, `+24V`, `0V`, …) are potential symbols from the *Potentials*
  library; *Project › Potentials…* defines their kind, voltage, wire colour and
  cross-section. The *Unplaced* tab lists defined potentials not on any page yet.
- Interruption points carry a net across pages; their cross references are set in
  *Project › Cross references…* (star or chain).
- *Project › Wire numbering…* numbers the connections (per potential, per page,
  formats such as `{page}.{n}`); a number typed into a wire's properties is kept.

## 5. Parts

- *Project › Parts management…* opens the parts database (product groups, filter,
  part editor with commercial, technical and typed mounting data, function templates,
  connection points, 2D / 3D models, assemblies). Parts are imported from CSV,
  BMEcat, EPLAN parts XML or the data portal (*Project › Data portal…*).
- Assign parts in the property panel of a device (*Add part…*); the device selection
  offers only parts that provide the device's main function. *Apply designations*
  takes the connection point designations from the part.
- The *Unplaced* tab then lists the functions of the assigned parts that are not in
  the schematic yet (for example the reserve contacts of a contactor); drag one into
  the page to place it with its tag and designations.

## 6. Terminals and plugs

- Terminals are placed like other symbols (*Terminals* library); terminals with the
  same tag form a strip. *Project › Terminal strips…* orders them, adds spares,
  jumpers (plug-in, saddle, wire), end brackets and plates and multi-level blocks.
- The *Strips / plugs* tab shows each strip in rail order and each plug with its pins;
  a click shows a placed terminal or pin, spares and unused pins are dragged into the
  page.
- *Project › Cables…* defines cables; select wires and use *Assign to cable*, or draw
  a cable definition line across them.

## 7. PLC

- PLC inputs and outputs are placed from the *PLC* library with their card tag and
  address. *Project › PLC hardware and bus topology…* adds stations, racks and modules
  from device descriptions (GSDML, ESI); *Project › PLC overview…* lists all points
  with their connected devices and readdresses cards.
- The *PLC* tab of the navigator shows the cards with their channels; free channels
  of cards with a device description are dragged into the page.
- *File › Export* writes the symbol table for the PLC programming tool: TIA Portal,
  Studio 5000, STEP 7 classic, TwinCAT 3, Control Expert, GX Works or IEC 61131-3.

## 8. The project check

- The *Check* tab runs the checks continuously (devices, connections, terminals,
  cables, PLC, panel layout, …). Click a message to jump to its place.
- Right click a message to accept it as checked, to change the level of its kind or to
  switch its check off; the settings (⚙) list them and exchange check schemes with
  other projects.

## 9. Reports

- *Reports* shows every report (device list, bill of materials, terminal diagram,
  cable diagram, plug diagram, PLC assignment, connection list, …) as a table with
  export to CSV and Excel.
- *Page › New report page* inserts a report as pages of the project; they update when
  the project changes. *Project › Title blocks, plot frames and page templates…*
  chooses the columns of a report form or switches the terminal, cable and plug
  diagrams and the PLC assignment to their graphical form.
- The bill of materials sums all parts of the devices, terminals, cables, harnesses,
  panel layout material, ferrules and marker material; assemblies list their
  components.

## 10. Panel layout

- The *Panel layout* workspace (toolbar) places enclosures, mounting plates, rails,
  ducts and the devices of the schematic; the 3D view shows the enclosure with the
  door, routed wires and duct fill.
- The *Wiring* workspace is the wiring assistant for the workshop (wires in order with
  colour, cross-section, length and end treatment); its *Mounting* switch is the
  mounting assistant (items in mounting order with place, size and part).
- Drilling patterns, NC data and the panel layout in 3D (STEP, glTF, JT, IFC) are
  exported from the enclosure properties and *File › Export*.

## 11. Output

- *File › Export* lists all exports: PDF (all pages or a page selection such as
  `1-3, 5`, with bookmarks and clickable cross references), HTML for printing, DXF /
  DWG, SVG, the production wire list, marking (with labeling schemes for marking
  programs), harness KBL and nailboard drawings, PLC tables, AutomationML and 3D
  formats. Exports with options ask for them first.
- *File › Print* opens all pages or the current page in the browser with its print
  dialog.
- *File › Export as PDF in language* writes the PDF in one of the project languages
  (*Project › Project languages…*).

## 12. Changes and review

- *Project › Revisions…* creates revisions; changes since the last released revision
  are marked on the pages and listed with old and new values in the revision
  overview. Released pages are locked.
- *Project › Review: comments and markup* and the *Review* tab hold comments and
  redline / greenline markup; *Import PDF comments…* takes the comments a reviewer
  made in a PDF export.

## 13. Automation

Everything above is also available without the user interface: the command line
(`cargo run -p openecraft-cli -- help`) and the MCP server for AI agents
(`openecraft-cli mcp`) offer the same commands, checks, reports and exports, and
plugins add their own (see [plugins.md](plugins.md)).
