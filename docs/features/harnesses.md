# Wire harnesses

Harnesses with 3D topology, protections and fixings, KBL export, the harness interface
for machines and nailboard drawings (*Connections › Harnesses…*). The example project
`conveyor` has a door harness.

## Harnesses and their wires

Single point-to-point connections (`-S1:13 – -K1:A1`) are assigned to harnesses
(`-WH1`), independent of the other connections of the same potential section;
assignments follow renamed devices, missing connections are reported by the check. The
*harness data* report lists their wires with length (routed in the panel layout or the
harness's field length) and end treatment, the harness window shows the connectors with
their pin assignment and the bundle diameter.

## 3D harness geometry

Nodes and segments with centre curves derived from the 3D routes of the panel layout
(connectors outside it placed around a common node), or stored in the harness and
edited (positions, bend radii, formboard lengths, control points); every wire is routed
through the segments, each segment has its bundle diameter; tape, tubes, corrugated
tubes, sleeves and heat shrink on (parts of) segments, clips, cable ties, grommets and
brackets at locations; the check reports broken topologies and too small bend radii;
export of the bundle in 3D (glTF, JT).

## KBL

VDA 4964, schema KBL 2.4 SR-1, checked against the schema in the tests: one container
per harness with general wires and cables (cores), connector housings with slots and
cavities, terminals, seals and cavity plugs, wire protections, fixings, accessories,
co-pack parts, assemblies and components, the connections between contact points,
schematic connections (potentials), wiring groups, project options as modules,
revisions, approvals, units, default tolerances and the 3D topology (exporters `kbl`,
`kbl_zip`).

## Harness interface

For machines and tools ([harness-interface.md](../harness-interface.md)): the complete
harness data as JSON (`harness_json`, command `harness_data`) and the wire processing
list for cutting, stripping and crimping machines (`harness_processing`, CSV in
processing order with cut lengths, stripping lengths, terminals and seals); the plugin
*Harnesses* edits topology, protections, fixings, end treatment and further parts from
the command line and MCP.

## Nailboard drawings

Formboards to scale: the bundle as branches from a common node to the connectors, each
as long as the wire lengths require and as thick as its bundle, with nails, branch
lengths and the pin table of every connector, on A3 at the smallest standard scale that
fits (harness window › *Nailboard (SVG)…*, exporter `nailboard`, command
`harness_nailboard`).
