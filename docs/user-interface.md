# User interface

The editor's layout and the commands and keys of everyday work. New users start with the [tutorial](tutorial.md).

The editor is laid out like EPLAN: a **ribbon** with the File tab (backstage: new,
open, save, import, export, print, settings) and the tabs *Home*, *Insert*, *Edit*,
*View*, *Devices*, *Connections*, *Project data*, *Tools*, *Master data*, *Plugins* and
*Help*, plus the context tabs *Layout space* (panel layout) and *Wiring* (assistants);
a **quick access row** above it (save, undo list, page switch, workspace, command
search); dockable **panes** for the page, the navigators, the Insert center, the
properties, the messages and every tool; a **status bar** with position, grid, layer,
zoom, language, check results and the messages of the session.

| Action | How |
| --- | --- |
| Find any command | `Ctrl+Shift+P` / `F3` or the search box of the quick access row: every command of the editor and of the plugins by name, tooltip or place |
| Key tips | `Alt` (or `F10`) shows a letter on each tab, a second letter runs a command; `Esc` goes back |
| Place symbol | *Insert › Insert center* (`Ins`): search (`*` and `?` as placeholders), favorites, recently used, symbols, macros, devices from parts, unplaced functions; click, then click into the page; `R` rotate, `M` mirror, `Esc` stop |
| Draw wires | `W` (*Insert › Wire*), click start and corner points, ends automatically on a connection point; `Space` flips the corner |
| Select / move | click, `Shift`+click, drag on empty space for window selection, drag items to move; right click for the commands of the selection |
| Edit properties | *Properties* pane with the tabs *Device*, *Display*, *Parts*, *Properties*, *Connections*; `F2` or a double click shows them in a window; several symbols: their common fields |
| Page structure | *Home › Page properties…* (or right click on a page): function (`=`) and location (`+`) |
| Page tabs | the pages opened lately are tabs above the page; `Ctrl+Tab` / `Ctrl+Shift+Tab` switch, `Ctrl+W` closes |
| Cables | *Connections › Cables…*; select wires › *Assign to cable*; single wire: cable and core |
| Wire numbers | *Connections › Wire numbering…*; single wire: type a number (kept by automatic numbering) |
| Macros | select items, right click › *Save selection as macro…*; place them from the Insert center or *Insert › Macro…* |
| Configurator | *Project data › Configurator…*: options, rules with page macros, *Generate pages* |
| Bulk changes | *Project data › Edit in table…* (or the table button of a navigator): devices, connections, terminals, cables, plug pins, PLC points, pages and parts; filter, edit, fill, find and replace, CSV |
| Project check | *Messages* pane (`F4`): sortable table of the messages; click shows the object, right click accepts, changes the level or switches the check off |
| Enclosure climate | panel layout: select the enclosure › *Climate (IEC 60890)* |
| Terminal strips / PLC | *Devices › Terminal strips…* / *Devices › PLC overview…* |
| Plugs | place pins from the symbol group *Plugs*; *Devices › Plugs…* for pin order and both sides |
| Revisions | *Project data › Revisions…*: create (and release) revisions, approval workflow, lock / unlock pages, see changes |
| Compare | *Project data › Compare and baselines…*: create baselines, compare, take differences over |
| Review | *Review* workspace or navigator, *Insert › Comment* and *Insert › Markup* (redline / greenline) |
| Settings | *File › Settings*: project settings (wire numbering, cross references, checks, marking, settings scheme) and user settings (appearance, UI scale, language, keyboard shortcuts, ribbon, servers) in one dialog |
| Settings scheme | *Project data › Export settings… / Import settings…*: the project's wire numbering, potentials, wire defaults and colours, marking, hose defaults, cross references and check scheme as one file for other projects (tools `export_settings`, `import_settings`) |
| Panes | drag the tab of any pane (page, navigators, Insert center, properties, messages, tools) beside, above or below another one or among its tabs; tools (reports, cables, revisions, parts, …) open as floating windows, *Dock* in a window puts it into the workspace and the pane menu (`⋯`) floats a docked pane again or closes it (the choice is remembered); *View › Panes* shows or hides each and resets the layout; `F6` moves between the panes |
| Workspaces | *View › Workspaces* or the chooser in the quick access row: *Schematic*, *Panel layout*, *Wiring*, *Mounting*, *Review*, *Pre-planning*, each with its own arrangement of the panes; *Save workspace…* keeps the arrangement and the ribbon under a name (kept by the app) |
| Ribbon | double click a tab or the arrow at the right minimizes it; *File › Settings › Ribbon* hides groups, adds own groups of commands and exports / imports them |
| Keyboard shortcuts | *File › Settings › Keyboard shortcuts*: record, clear or reset the keys of every command; conflicts are shown, the keys are kept by the app and shown in the tooltips and the help |
| Change requests | *Project data › Change requests…*: requests, CSV / JSON import and export |
| Export | *File › Export*: PDF, DXF / DWG (page / all), SVG, HTML, production wire list, panel layout in 3D (glTF / STEP / IGES / IFC / JT / Parasolid) |
| Import old drawings | *File › Import › Import DXF / DWG as page…* |
| Own symbols | *Master data › Symbol editor…*; they appear under *User symbols* in the Insert center |
| Title blocks / report forms | *Master data › Title blocks, plot frames and page templates…*; per page in the page properties |
| Panel production | *Layout space* tab: *Busbar*, *Cutout*, *NC data* (DXF / DWG / CSV) |
| Door devices | *Layout space › Door (front)* for door devices and cutouts, *Door (inside)* (mirrored) for cable ducts and the door wiring; 3D: *Open door* / *Close door* |
| Wiring the panel | *Wiring* workspace: pick a wire in the list or in 3D, *Wired* / *Problem*; *Wiring › Tablet mode* for touch screens; web: open with `#wiring` |
| Project texts in several languages | *Project data › Project languages…*: target languages, *Collect texts from the project*, translate (or CSV for translators); *Project data › Export as PDF in language* |
| Hoses / PCT loops | draw wires between fluid / P&ID ports; hose properties and configuration in the wire properties; *Tools › Fluid and process…* (hoses, device groups, electro-fluid links and *Synchronize*, PCT loops) |
| Harnesses | *Connections › Harnesses…*; select the wires between two devices › *Assign to harness*, or per connection in the wire properties; 3D topology, protections, fixings, end treatment, KBL, harness interface and 3D export in the pane |
| Help | `F1` opens the help of the pane in use: quick guide, tutorial, keyboard shortcuts; *Help › Start page* |
| Pan / zoom | right or middle mouse drag / mouse wheel, `F` fits the page |
| Pages | `Page Up` / `Page Down`, *Pages* navigator (its context menu also adds a schematic page after the page, renumbers and deletes pages), page switch of the quick access row |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y`; the undo list of the quick access row undoes several steps |
| Copy / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V`, *Home › Cut* / *Copy* / *Paste* |
