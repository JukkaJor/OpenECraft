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
  rooms and enclosures with content are deleted only after a question.
- **Engineering:** conductor sizing, short circuit and selectivity, load calculation,
  EMC, ISO 13849-1 safety functions; wire harnesses with KBL (their wires stay assigned when pages or symbols move), fluid
  power, pre-planning.
- **Revisions and review:** released pages, approval workflow, baselines with compare
  and merge, comments and redlining.
- **Parts:** a parts database with mounting data and 3D models (STEP, IGES, Parasolid,
  JT, IFC), BMEcat, EPLAN XML and EDZ import; a parts server with a data portal that
  shows the 2D front view and the 3D model of a part; the example parts, mounting
  material included, come with both. Parts are dragged from parts management or the
  data portal onto a schematic page, imported into the parts database on the way when
  needed, and placed as a device with the part.
- **Data exchange:** PDF, DXF / DWG, SVG; the panel as glTF, STEP, IGES, IFC, JT and
  Parasolid; ERP / PDM / PLM connectors and Autodesk Vault.
- **Teams and automation:** a project server with page locks, roles and Active
  Directory / LDAP sign-in; a command line, an MCP server for AI agents and plugins.
- **Languages:** English, German, French, Spanish and 16 more.
- **Packages:** Windows setup, deb, rpm, AppImage, Flatpak, snap, tar.gz, macOS disk
  image and the web version.
