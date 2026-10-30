# Parts management

The parts database shared by all projects (EPLAN style): commercial, technical and
mounting data, function templates, connection points, 2D / 3D graphics and the import of
manufacturer data. Parts are shared between installations through the
[parts server](../parts-server.md).

## Parts database

A global **parts database** shared by all projects (product top group / group /
subgroup tree, filter), with general, commercial (price, price unit, packaging, quantity
unit), technical, mounting (dimensions, weight) data, accessories and a **function
template** per part (e.g. coil A1/A2, main contacts, 13/14 NO). The database is stored
in the app storage (desktop) / local storage (browser).

- **Assemblies**: a part can consist of component parts with quantities (tab
  *Accessories*, CSV column `Components`: `NUM:2|NUM2`); the bill of materials lists
  them under the assembly (nested assemblies resolved, priced as included).
- **Typed mounting data** per part (tab *Mounting*): clearances (top, bottom, left,
  right, front, EMC, service space), mounting holes with thread, door and face cutouts,
  power loss, how far an accessory sticks out, fastening, faces, collision class and
  terminal type. They drive the panel checks, drilling, cutouts and the climate
  calculation; CSV has a column per value (`Power loss`, `Mounting holes`, …). Older
  parts with these values as `Key: value` lines in the technical data still work;
  imports and the built-in catalog move such lines into the typed fields, the part
  editor offers it for existing parts.
- **Connection points** per part (designation, position in the front view, the
  direction the wire leaves); parts without their own get them from the function
  templates by the IEC convention (odd designations at the top, even at the bottom, door
  devices from the rear, PLC modules at the bottom). Edited in the part editor (tab
  *Connection points*: table, click into the front view to place one); the panel layout
  wires to them; CSV column `Connection points`: `A1:22.5/0/55/up|A2:…`.

## Graphics and 3D models

2D front view (SVG) and 3D model per part:

- **STL**, **OBJ**;
- **STEP**: solids and surface models of AP203 / AP214 / AP242 files, placed by their
  assembly structure, tessellated and converted to mm;
- **IGES 5.3**: solids and trimmed surfaces;
- **Parasolid** (`.x_t` / `.x_b`): bodies of V13 and newer transmit files, planes,
  cylinders, cones, spheres, tori and B-surfaces tessellated exactly, blends and other
  procedural faces filled from their boundary;
- **IFC**: tessellated, faceted and extruded bodies of all products, placed and
  converted to mm;
- **JT**: the tri-strip meshes of JT 8 files.

Parts with dimensions but without a model get a generated one by what they are: DIN
rail devices with their terminals, toggle or display, terminals in their colours (PE
green, N blue), PLC modules, fuse switch disconnectors, wall-mounted electronics with
display and cable glands, 19-inch devices and rack batteries, PV modules, pumps, motors,
vessels, heat pumps, compressors, refrigeration dryers, valves with lever or actuator,
pipe fittings, manifolds and air treatment units. Devices of the bundled catalogs
are drawn after their product photos: Victron inverters with their black corner caps
and orange band, the Cerbo GX and SmartShunt, the Lynx distributor, Pylontech rack
batteries with handles and terminals, Hager fuse switch disconnectors, meters and
modular devices, DEHN arresters with their plug-in modules, Beckhoff EtherCAT
terminals, Phoenix Contact power supplies, Siemens motor protection switches and
contactors. Generated models are painted (a colour
per triangle, `colors` of the mesh); imported ones take the colour the 3D view gives
the device. Floor-standing parts stand upright on the floor of a room.

## Parts in the project

- **Project parts**: parts are copied into the project when assigned; synchronize
  project ← database, transfer project → database, remove unused project parts.
- **Part selection** dialog with *device selection* (only parts providing the device's
  main function); several parts per device with quantity (contactor + aux block).
- Function templates drive **reserve contacts** in the contact image, "Apply
  designations" (connection point numbers from the part) and the project check (more
  functions placed than the parts provide, parts missing in the project).

## Import and export

- **CSV** (`;` separated, header names) and **JSON**, import and export.
- **BMEcat** catalogs (1.2 / 2005, as published by manufacturers and wholesalers, e.g.
  from the EPLAN Data Portal or ETIM / eCl@ss data pools): identification, description,
  price, data sheet, the ETIM / eCl@ss class and every feature as technical data;
  dimensions, weight, rated current / voltage and power loss go into their fields (by
  feature name, or the ETIM codes of width, height and depth).
- **EPLAN parts XML**: the parts management export of EPLAN (`<part>` with
  `P_ARTICLE_*` attributes and variants, multi-language texts, product groups by their
  exchange numbers) is imported like the other formats, and parts export as EPLAN parts
  XML for the way back (parts management, `parts_export --format eplan_xml`). Function
  templates become function templates (PLC inputs and outputs as PLC functions),
  connection point definitions connection points (connection points on plugs as
  `X10.1`), drilling patterns mounting holes.
- **EDZ** (EPLAN Data Portal archives, 7-Zip): the parts XML inside is read as above,
  together with clearances, power loss and the datasheet link. Parts management and the
  editor import the parts; the parts manager and `parts_import` also store the picture
  and the EPLAN macros (`.ema`, kept as they are, not shown) with the part on the parts
  server.
