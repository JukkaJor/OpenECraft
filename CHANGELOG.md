# Changelog

The notable changes of each OpenECraft release. The section of a version is the text of
its GitHub release ([docs/releasing.md](docs/releasing.md)).

## [0.1.0] - unreleased

The first release: an electrical CAE editor for Windows, Linux, macOS and the browser,
with a command line, an MCP server, a parts server and a project server.

- **Schematics:** device-oriented pages with live cross references and contact images,
  structure identifiers, page sections, macros with variants and a configurator;
  potentials, cables with cores and automatic wire, device and page numbering.
- **Symbols:** about 330 IEC 60617 symbols, NFPA / JIC, GB, GOST and JIS sets, fluid
  power (ISO 1219), P&ID, installation plans and single-line; a symbol editor.
- **Terminals, plugs, PLC:** multi-level terminals with accessories, plugs, PLC tag
  tables for TIA Portal, Studio 5000, TwinCAT, CODESYS and more, marker export.
- **Reports and checks:** live report pages, graphical terminal, cable and plug
  diagrams, drawn forms, a project check with message management and a check for
  open wire ends.
- **Editor:** an EPLAN-style ribbon that folds groups into menus when space is short,
  dockable panes per workspace with standard workspaces to return to their default
  arrangement, an Insert center, a properties pane, key tips,
  command search; unsaved changes are kept and asked for on every way out; released
  and locked pages are read only but selectable; crossing selection, zoom commands,
  contextual help (F1) and rebindable keys for every page action; pages and sections
  are dragged in the Pages navigator with a drop marker and the resulting name shown.
- **Panel layout:** devices at real size in 2D and 3D, virtual wiring with duct fill,
  drilling and NC data, enclosure climate, line-ups, rooms and 3D routing; wiring and
  mounting assistants for the workshop. In a room's floor plan cabinets turn in place,
  cabinets in a bad place are drawn in the warning colour with the reason on hover,
  inner walls end on the room walls, and the layout checks are shown below the view;
  rooms and enclosures with content are deleted only after a question. Opening a project
  analyses it in the background with a window showing the step and the route it is
  at; the 3D routing searches several times faster. Pipes and cables in a room keep to
  its walls and ceiling instead of crossing it, and their bends keep the bend radius
  apart. The 3D view is drawn by the graphics card (OpenGL 3 / WebGL 2, with a depth
  buffer and anti-aliasing; Settings › Appearance switches it off), and drawn by the
  processor it keeps what it shows between frames: large rooms turn smoothly. Devices without a 3D
  model of their own get a detailed, coloured one by what they are (pumps, vessels,
  valves, heat pumps, inverters, rack batteries, …); devices of the bundled catalogs
  (Victron, Pylontech, Hager, DEHN, Beckhoff, Phoenix Contact, Siemens) are drawn after
  their product photos. The 3D view hides the walls in front of the camera or the
  ceiling on request; the opacity of the walls is a setting. Wires are drawn in their
  colours and, like cables and tubes, in their real thickness. The title bar can show
  the frame time and the frames per second (Settings › Appearance). A placed device is shown in the
  panel layout or the 3D view from its context menu in the schematic. Pumps, tanks
  and other parts marked for floor mounting stand on the floor of a room, pipe runs
  go on its walls, ceiling and floor, and hoses, pipes and cables are routed in 3D
  through the room and from its cabinets through their cable glands. Wall
  penetrations join neighbouring rooms: routes run from room to room through them
  (also through the rooms between), and the floor plan shows the rooms around the one
  shown, opened with a double-click. **19-inch racks** (42 U in 2000 mm) have mounting
  rails instead of a plate; switches, patch panels, servers, UPS and blank panels snap
  onto whole rack units, with a rack elevation report and the commands `add_rack`,
  `mount_in_rack` and `list_racks`. **Building entries** in outer walls bring the
  supply cable and the water main into the building: cables and pipes to devices
  outside the building are routed to them. Of several wall penetrations into the same
  room a route takes the nearest; cables and pipes ending at the same device (a heat
  pump) may meet there.
- **Engineering:** conductor sizing (`size_wires` gives the wires behind protective
  devices the cross-section they need; fuse switch disconnectors protect at the rating
  of their fuse links), short circuit and selectivity, load calculation,
  EMC, ISO 13849-1 safety functions; wire harnesses with KBL (their wires stay assigned when pages or symbols move), fluid
  power, pre-planning. Fluid power and P&ID symbols for air receivers, dryers,
  lubricators, quick couplings, heat pumps and buffer tanks, the media hot water and
  heating water; symbols for energy meters, PV generators, hybrid inverters, inverter /
  chargers, valve actuators and motor-operated valves. A device drawn pole by pole (fuse
  holders, surge arresters, fuse blocks) may have as many main functions as its parts
  provide.
- **Revisions and review:** released pages, approval workflow, baselines with compare
  and merge, comments and redlining.
- **Parts:** a parts database with mounting data and 3D models (STEP, IGES, Parasolid,
  JT, IFC), BMEcat, EPLAN XML and EDZ import; a parts server with a data portal that
  shows the 2D front view and the 3D model of a part; the example parts, mounting
  material included, come with both; the manufacturer catalogs of the parts server
  (ABB, Beckhoff, DEHN, Hager with NH fuse switch disconnectors and energy meters,
  MDT, Phoenix Contact, Pylontech, Siemens, Victron, Weidmüller) also feed the example
  projects. Parts are dragged from parts management or the
  data portal onto a schematic page, imported into the parts database on the way when
  needed, and placed as a device with the part.
- **Data exchange:** PDF, DXF / DWG, SVG; the panel as glTF, STEP, IGES, IFC, JT and
  Parasolid; ERP / PDM / PLM connectors and Autodesk Vault.
- **Teams and automation:** a project server with page locks, roles and Active
  Directory / LDAP sign-in; a command line, an MCP server for AI agents and plugins.
- **Languages:** English, German, French, Spanish and 16 more.
- **Packages:** Windows setup, deb, rpm, AppImage, Flatpak, snap, tar.gz, macOS disk
  image and the web version.
