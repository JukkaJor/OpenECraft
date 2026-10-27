# Terminal strips, plugs, PLC and marking

Terminal strips with multi-level terminals and accessories, plugs, PLC cards with the exchange to PLC programming tools, and the markers for the workshop.

## Terminal strip editor

Terminal order (move up / down, sort), renumbering 1 … n,
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

## Plugs

**Plugs** (*Devices › Plugs…*, symbol group *Plugs*): pin and socket (mated pair, two
connection points), male pin and female socket (one each); tag = plug (`-X10`),
designation = pin. Male and female pins of the same designation are mated: the
potential passes through, the two sides are separate connections (wire numbers,
cables). Pins are numbered per plug when placed; the plug window and the *plug
diagram* report list the pins in plug order with the socket side, the pin side and
the potential; parts with `Plug pin` functions give the pin count (unused pins are
listed as spare). The check reports pins placed twice and more pins than the plug
parts have; the KBL export names the housing part of plug connectors with all
cavities of the part.

## Strip and plug navigator

**Strip and plug navigator** (navigator tab *Strips / plugs*): the terminal strips with
their terminals in rail order (levels of multi-level blocks) and spares, the plugs with
their pins and targets and the unused pins of the plug parts; a click shows a placed
one, spares and unused pins are dragged into the page.

## PLC navigator

**PLC navigator** (navigator tab *PLC*): the cards with their channels in address
order, placed ones with function text and connected devices (a click shows the
symbol), free channels of cards with a device description to drag into the page.

## PLC overview

All inputs / outputs per card with connected devices, editable
addresses and function texts, card start byte / rack / slot, readdressing, **symbol
table export / import** (CSV: name, data type, address, comment) for PLC programming
tools, the tag table of **Siemens TIA Portal** (Excel sheet *PLC Tags*), the tag
import of **Rockwell Studio 5000** (aliases of the module channels), the symbol table
of **Siemens STEP 7 classic** (SDF), a global variable list for **Beckhoff TwinCAT 3**
(TcGVL, `AT %I*` / `AT %Q*`), variables for **Schneider Control Expert** (XSY, `EBOOL`
with topological addresses), device comments for **Mitsubishi GX Works** (CSV, `X` / `Y`
in octal) and IEC 61131-3 global variables (CODESYS); addresses
used on several cards are reported.

## Marking

**Marking** (*Connections › Marking…*: wire marker text, material per kind with piece
counts and export; *File › Export › Export marking (CSV)…*, report *Marking*): device markers (legend
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
