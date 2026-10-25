# Harness data: KBL, machine interface, 3D geometry

OpenECraft hands harnesses to manufacturing and to other tools in three ways:

| Export | Name | Content |
| --- | --- | --- |
| KBL (VDA 4964) | `kbl` (one harness), `kbl_zip` (all) | The harness as KBL 2.4 SR-1 container |
| Harness interface | `harness_json`, command `harness_data` | The complete harness data as JSON (format `openecraft.harness`) |
| Wire processing list | `harness_processing` | One CSV row per wire for cutting, stripping and crimping machines |
| Bundle in 3D | `harness_gltf`, `harness_jt` | Segments as tubes of their bundle diameter, protections, fixings, connectors |

All of them are exporters of the built-in plugin `project_exports`, so they are in
*File › Export*, in the harness window, in `openecraft-cli export --format …
--options harness=-WH1` and in the MCP `export` tool. The commands of the built-in
plugin `harnesses` read and edit the harness data (see below).

## 3D topology

Each harness has a topology: **nodes** (connector positions, branch points) and
**segments** between two nodes with a centre curve (polyline, mm). Every wire is
routed through the segments from its first to its second end; a segment carries the
bundle diameter of its wires. The topology comes from, in this order:

1. the harness itself, if it is **stored** (`store_harness_topology`, harness window
   › *Store for editing*): nodes, segments, control points, bend radii and formboard
   lengths are edited there, wires take the shortest way through the segments;
2. the **panel layout**: the 3D routes of the harness wires (or of their cables) are
   traced through the routing grid; where wires run together they form a segment,
   where they part there is a node. Connectors outside the layout are placed around
   the common node at the branch length the wire lengths require;
3. otherwise a **generated** star around a common node, as on the nailboard.

Coordinates are those of the panel layout: x right, y down, z from the rear wall, in
millimetres. Wires that are not routed in the panel layout get their length from a
stored topology (the sum of the formboard lengths of their segments).

Protections (tape, tube, corrugated tube, braided sleeve, heat shrink) cover a part
of one segment (`start` … `end`, 0 … 1) or all segments; fixings (clip, cable tie,
grommet, bracket) sit at a location of a segment. The check *Harnesses* reports ids
used twice, segments at unknown nodes, protections and fixings on unknown segments,
connectors at no node, wires without a way and bend radii below the smallest bend
radius of the bundle (layout setting *bend radius of wires* × bundle diameter).

## Harness interface (JSON)

```json
{
  "format": "openecraft.harness",
  "version": 1,
  "units": { "length": "mm", "coordinates": "mm, panel layout: x right, y down, z from the rear wall" },
  "project": { "name": "…", "revision": "…", "company": "…", "date": "…" },
  "harnesses": [ {
    "tag": "-WH1", "description": "…", "part": "…", "field_length_m": 1.5,
    "wire_count": 14, "wire_length_mm": 17320.0, "bundle_diameter_mm": 10.3,
    "connectors": [ { "device": "-X1", "part": "…", "description": "…", "plug": true,
      "usage": "", "cavity_plug": "", "node": "N3",
      "pins": [ { "pin": "1", "wire": "17", "to": "-K1:A1", "terminal": "…", "seal": "…", "strip_mm": 8.0 } ] } ],
    "wires": [ { "number": "17", "net": "24V", "color": "BU", "cross_section": "0.75 mm²",
      "cross_section_mm2": 0.75, "outer_diameter_mm": 2.386, "cable": "", "core": "",
      "length_mm": 1500.0, "length_source": "panel | topology | field | unknown",
      "dmu_length_mm": 1450.2, "cut_length_mm": 1500.0,
      "from": { "device": "-S1", "pin": "13", "terminal": "GEN.FER-0.75", "seal": "", "strip_mm": 8.0 },
      "to": { … }, "segments": ["S1", "S4"] } ],
    "topology": { "source": "stored | layout | generated", "length_mm": 2310.0,
      "nodes": [ { "id": "N1", "position": [x, y, z], "connectors": ["-S1"], "bend_radius_mm": 0.0 } ],
      "segments": [ { "id": "S1", "start": "N1", "end": "N2", "length_mm": 158.0,
        "physical_length_mm": 158.0, "diameter_mm": 4.1, "points": [[x, y, z], …], "wires": ["17", "19"] } ] },
    "protections": [ { "id": "P1", "kind": "tape", "part": "…", "segment": "", "start": 0.0, "end": 1.0 } ],
    "fixings": [ { "id": "F1", "kind": "cable_tie", "part": "…", "segment": "S1", "location": 0.5 } ],
    "parts": [ { "kind": "accessory | co_pack | assembly | component", "part": "…", "count": 1, "description": "…", "node": "" } ]
  } ]
}
```

`length_mm` is the production length (routed in the panel layout including the
reserves, along the stored topology, or the field length of the harness);
`dmu_length_mm` is the length along the segments; `cut_length_mm` is the length
rounded up to the cut step of wires of the routing rules. Field names are stable
within a version; new fields may be added, a change of meaning raises `version`.

## Wire processing list (CSV)

Semicolon separated, the first row holds the column names (not translated):

`sequence;harness;wire;cable;core;color;cross_section_mm2;length_mm;cut_length_mm;from;strip_from_mm;terminal_from;seal_from;to;strip_to_mm;terminal_to;seal_to;marking`

The rows are in processing order: by cross-section, colour and the terminals and
seals of both ends (fewest changeovers), longest wire first. Terminals and stripping
lengths come from the end treatment of the harness (`set_harness_end`, harness window
› *End treatment of the connectors*); device ends without one get the ferrule of
their cross-section when the panel layout uses ferrules.

## KBL

The KBL file follows the KBL 2.4 SR-1 schema of prostep ivip (namespace
`http://www.prostep.org/Car_electric_container/KBL2.3/KBLSchema`, `version_id`
`2.4`): element order, required ids, enumerations and references are checked
against the schema in `crates/openecraft-core/src/kbl/tests.rs`. A container holds
one harness. What the project fills:

| KBL | From |
| --- | --- |
| `Harness` | harness tag, part number, description (translations of the project languages as `Localized_description`), company, revision, project name, model year of the project date, wire and housing mass, revisions as `Change` |
| `General_wire`, `Wire_occurrence` | cross-section and colour of the wires, outer diameter, bend radius; production, DMU or nominal length, marking |
| `General_wire` with `Core`, `Special_wire_occurrence`, `Core_occurrence` | cables whose cores are harness wires |
| `Connector_housing` with `Slots` / `Cavities`, `Connector_occurrence` with `Slots`, `Contact_points` | the devices at the wire ends: plugs with the cavities of their part (also unused ones), other devices with their connection points; placement at their node |
| `General_terminal`, `Terminal_occurrence`, `Cavity_seal`, `Cavity_seal_occurrence`, `Cavity_plug`, `Cavity_plug_occurrence` | end treatment of the harness, ferrules |
| `Connection`, `Extremities` | the harness wires between contact points, potential as signal name and nominal voltage, stripping length |
| `Schematic_connection` | the potentials and the cavities on them |
| `Wiring_group` | wires sharing a contact point |
| `Wire_protection`, `Wire_protection_occurrence`, `Protection_area` | protections |
| `Fixing`, `Fixing_occurrence`, `Fixing_assignment` | fixings |
| `Accessory`, `Co_pack_part`, `Assembly_part`, `Component` and their occurrences | further parts of the harness |
| `Module`, `Harness_configuration` | project options with devices or wires of the harness; the active ones form the configuration |
| `Cartesian_point`, `Node`, `Segment` (centre curve, vectors, lengths, cross-section area), `Routing` | the 3D topology |
| `Dimension_specification`, `Default_dimension_specification` | segment lengths with the default tolerances (≤ 500 mm ± 5, ≤ 1500 mm ± 10, ≤ 3000 mm ± 15, above ± 20) |
| `Unit`, `External_reference`, `Creation`, `Approval` | mm, mm², g; the project as schematic; creator; released revisions |

Not filled, because the project has no such data: component boxes (fuse and relay
boxes) and fuses, modular slots, special terminals (splices inside the bundle) and
the `Specified_…` occurrences of assembly parts.

## Commands

| Command | Does |
| --- | --- |
| `list_harnesses` | harnesses with wires, connectors, length, bundle diameter, topology source |
| `harness_data` | the harness interface document (one or all harnesses) |
| `harness_topology` | nodes, segments, the way of each wire, problems |
| `store_harness_topology`, `clear_harness_topology` | store the derived topology for editing, drop it |
| `set_harness_node`, `set_harness_segment` | add, change or remove nodes and segments (stores the derived topology first) |
| `set_harness_protection`, `set_harness_fixing` | protections and fixings |
| `set_harness_end` | terminal, seal, cavity plug, stripping length and KBL usage of a connector or pin |
| `set_harness_part` | accessories, co-pack parts, assemblies, components |
| `harness_nailboard` | the nailboard layout (plugin `wiring`) |
