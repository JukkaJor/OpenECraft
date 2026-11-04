# Wires, cables, potentials and numbering

How connections get their cable, colour, cross-section and number, and how devices, pages and wires are numbered.

## Potential definitions

**Potential definitions** (*Connections › Potentials…*): kind (L, N, PE, DC +/−, signal),
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

## Cables

Cable definitions (tag, type, cores, cross-section, length, part from the
product group *Cables*). Wires are assigned to cores (several wires at once get the free
cores in order, the wires of one connection share a core); cores and the cable tag are
shown on the schematic. Cable overview and
cable diagram reports, cables in the BOM by length, checks for unknown cables, cores
used twice or not in the cable.

## Connection and cable definitions

A connection definition point on a wire sets
number, colour and cross-section of the whole connection and the core of its wire; a
cable definition line drawn across wires makes them cores of a cable (new cable with
numbered cores, or a chosen one). The check reports points beside wires, contradicting
or not applied points, and lines of unknown cables.

## Wire numbers

One number per connection (all its wire segments; the two sides of a
terminal are separate connections). Automatic numbering: sequential, page/column
(`3.4.1`) or by potential name (`L1`, `+24V`), with prefix and start value; manually
set numbers are kept. Shown on the schematic, in the connection list and the panel
wire list; checks for conflicting and duplicate numbers.

## Numbering

**Numbering** (in the spirit of EPLAN's offline numbering): *Devices › Device
numbering…* renumbers device tags per structure and identifying letter in schematic
order (page, column, top to bottom) after a format: `{letter}{n}` (`-K1`),
`{letter}{page}.{n}` (`-K3.1`), `{letter}{page}{column}` (`-K34`) or NFPA style
`{page}{column}{letter}` (`-34K`), with start value, step, letters and scope, and a
preview of the new tags; everything that refers to a tag follows (parts, strips,
plugs, PLC data, panel layout, harnesses); tags of devices that are not placed
(device list, panel layout) are not given to others. *Home › Renumber pages…* numbers pages
with start and step, restarting per structure and keeping sub-pages (`3.a`; *Home › New page › New
sub-page* inserts `3.1`, `3.2`, … after the current page). Page sections hold pages
and subsections like directories (*Home › New section*, drag pages onto them in the
*Pages* navigator); the page names carry the section numbers (`2.3.1`), follow when a
section is renumbered or moved, and renumbering counts per section. The wire
numbering gets a *Format* scheme with placeholders (`{page}`, `{column}`, `{row}`,
`{potential}`, `{kind}`, `{color}`, `{cross_section}`, counter `{n}` counted per
value of the rest), counter digits and step, and potential kinds that get no number
(e.g. PE). Commands: `renumber_devices`, `renumber_pages`, `set_wire_numbering`.
