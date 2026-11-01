# Panel layout, rooms and 3D routing

Enclosures with their devices at real size in 2D and 3D (in the spirit of EPLAN Pro
Panel), virtual wiring through the ducts, drilling and NC data, enclosure climate,
line-ups, rooms of the building with their floor-standing equipment and pipe runs,
19-inch racks and 3D routing of cables, tubes and pipes. The example projects `conveyor`,
`cabinet_lineup`, `electrical_room` and `pump_room` show them.

![The line-up of the cabinet_lineup example in 3D, door open, with rails, ducts, devices and routed wires](../images/panel-3d.png)

## Enclosures and devices

- Enclosures (from a part such as `GEN.ENC-600X800` or custom size) with mounting plate.
- Mounting rails and cable ducts drawn by length; devices placed from a "devices to
  place" list with their real dimensions from the parts data (contactor + auxiliary
  block add up in depth, terminal strips are as long as their terminals and spares) and
  snapped onto rails; move (devices travel with their rail), rotate, delete.
- 2D front view and a 3D view (orbit, pan, zoom, click to select). Walls, ceilings and
  doors seen from the outside are see-through (*Wall opacity* in the settings, under
  Appearance); *Hide walls in front* leaves out the walls between the eye and the inside,
  *Hide ceiling* the ceilings of the rooms. Wires are drawn in their colour (two-colour
  wires such as green-yellow with a stripe) and, like cables and tubes, as thick as
  their outer diameter. A cable ends a little before the device it is connected to and
  its cores part from there in smooth curves, each into its connection point from the
  side its wires leave it; through cable glands, wall penetrations and the cable passages
  between cabinets cables and wires are drawn in one piece, bent round there, several
  through one gland side by side. Without cable ducts a wire leaves its connection point
  in the direction it points, runs round the rows of devices (25 mm clear of them) and
  never behind a device; a short jumper stays at the height of its connection points.
- Copper **busbar systems** (bars drawn, metres and holders in the BOM) and rectangular
  or round **cutouts** in the mounting plate. A device placed over a busbar system is
  mounted onto its bars; devices on the same busbar system are connected by the bars,
  not by wires.
- Mounting layout pages (scaled 2D model view with dimensions) for printing / SVG;
  rails and ducts appear in the BOM by cut length, enclosures per piece.
- Jump between a device in the schematic and its place in the panel layout.

## Collision and boundary checks

Overlaps, outside the plate, too deep for the enclosure including a door clearance,
mounted over a cutout, inside the clearance a device needs above and below from its part
data (`Clearance: 25 mm`); where a device has a 3D model, the **models** are checked for
penetration, not just their boxes.

## Enclosure door

Command and signal devices (pushbuttons, lamps …) are placed on the door (front view
*Door*); each gets its **door cutout** (Ø22.5 by default or from the part data
`Door cutout: Ø30.5` / `92 x 68`), checked against the door frame and in 3D against the
equipment of the mounting plate; their wires are routed over the hinge; the mounting
layout page shows the plate and every face with items (door, side walls, rear wall,
roof, bottom) side by side.

## Drilling pattern and NC data

Rails, ducts and busbar holders get tapped holes along their length, devices screwed
onto the plate get holes from their part data (`Mounting holes: M5: 9,12; 69,72`) or two
default holes; **NC export** per enclosure and face as DXF or DWG (outline, one layer
per hole diameter, cutouts) or CSV.

## Virtual wiring and duct fill levels

Every schematic connection between placed devices is routed from its connection point
(rotated devices, aux blocks, terminal by terminal) into the duct the terminal faces and
through the cable duct network (shortest path) → wire list with lengths, wires shown in
2D / 3D.

**Duct fill levels**: cross sections of the routed wires (with insulation), cables and
tubes against the cross section inside the duct (between its walls, base and cover) at
its fullest stretch; ducts coloured green / orange / red in the
front view, fill level in the duct properties, limit in the enclosure properties (70 %
by default); report *Duct fill levels*, the check warns above the limit and hints above
80 % of it; when a duct is overfull, the wiring is routed again, thickest wires first,
and wires take a detour (up to 10 × longer) around stretches they would fill above the
limit; cables and tubes routed in 3D do the same around the stretches the wires and the
cables before them already fill; the selected device shows its connection points.

## Climate

IEC/TR 60890, in the enclosure properties: power loss of the placed devices from the
tech data of their parts (`Power loss: 7.5 W`) plus an additional loss, effective
surface by installation type (single, in a suite, free-standing, wall mounted, roof
covered), material (heat transfer coefficient), ambient and allowed inside temperatures
→ inside temperature, filter fan air flow (corrected for the altitude of the site),
air/air heat exchanger (W/K, when the degree of protection must be kept) or cooling unit
output, heater output that also keeps the inside above the dew point of the ambient air
(relative humidity); report *Enclosure climate*, the check warns when cooling is needed.

The heat map of the mounting plate stratifies the mean temperature and adds a plume
above every device with power loss data; horizontal ducts hold plumes back (warm air
gathers below them), fans and cooling units weaken them.

![Heat map of a mounting plate beside the climate calculation of the enclosure](../images/heat-map.png)

## Line-ups

Enclosures bayed side by side with cable passages between them (cutouts in the facing
side walls, or sides open to each other); the 3D view shows the whole line-up, wires run
from one enclosure into the next. The *room* view shows all enclosures side by side at
their places, each line-up at its position and turn set by hand or taken from the plant
layout.

## Rooms

*Layout space › New room* (plugin `rooms`): a room of the building with its floor plan,
height and place in the plant. The floor plan is a rectangle or an outline of walls at
right angles (L, U or T shaped rooms): *Floor plan* shows it from above, *Draw walls*
draws it by clicking the corners (on a 50 mm grid, closed by clicking the first corner
or Enter), walls are dragged to move them or get a length in the room's properties
(commands `set_room_outline`, `set_room_wall`).

- **Inner walls** (*Layout space › Draw › Inner walls*, commands `add_inner_wall`,
  `set_inner_wall`) are drawn in the floor plan by clicking their ends (a chain, at
  right angles; an end near a wall snaps onto it), dragged across to move them, get
  length and thickness in the room's properties and are removed with Del; both sides (`inner1a`, `inner1b`) are faces like
  the walls.
- **Doors and windows** (*Layout space › Draw › Door / Window*, command
  `add_wall_opening`) are openings in a wall, placed by clicking the wall in the floor
  plan or in the view of the wall: a door stands on the floor and swings into the room
  (drawn with its swing), a window has its sill height; size, position and sill are in
  their properties.
- **Cabinets** (their line-ups) stand in it at a place of the floor plan, dragged there
  in the floor plan (R turns the selected one about its centre), and move with the
  room. A cabinet outside the room, across a wall, in an inner wall or in the swing
  of a door is drawn in the warning colour, and the pointer over it tells why.
- Every wall (seen from inside the room) and the ceiling carry mounting rails, cable
  ducts and devices: a device on a wall sits on a rail, unless its part is marked
  *Wall mounting* (part editor › Mounting data, a `WALL` badge in the part trees, filter
  *Only parts for direct wall mounting*), then it goes directly on the wall
  (*Insert › Wall-mounted part*, also without a device tag).
- The **floor** (face `floor`) carries the parts marked *Floor mounting* (pumps, tanks,
  compressors; a `FLOOR` badge, filter *Only floor-standing parts*): *Insert ›
  Floor-standing part*, devices to place on the floor's view, command `mount_on_wall`
  with `face` `floor`. They are drawn in the floor plan, dragged there to move them and
  removed with Del.
- **Wall penetrations** (*Layout space › Draw › Wall penetration*, command
  `add_wall_opening` with `kind` `penetration`; 300 x 200 mm, 300 mm below the ceiling
  unless set otherwise) go into the outer walls and lead into the room behind the
  wall (up to 1 m away, through rooms turned against each other too): cables, tubes
  and pipes are routed from room to room through them. The floor plan draws them and
  the way through the wall dashed; `list_rooms` tells where each one leads.
- **Building entries** (*Layout space › Draw › Building entry*, command
  `add_wall_opening` with `kind` `entry`; 200 x 200 mm, 600 mm above the floor unless
  set otherwise) are the house lead-ins of the outer walls: the supply cable, the
  water main and other lines from outside the building come in through them. A cable
  or pipe from a device in the room to a device outside the building (its location is
  none of the rooms and enclosures of the layout, such as the utility's cable
  distribution cabinet) is routed to the nearest building entry.
- **Pipe runs** (*Layout space › Draw › Pipe run*, command `add_pipe_run`) are drawn on a
  wall, the ceiling or the floor by clicking their corners (at right angles), 100 mm in
  front of a wall, 300 mm below the ceiling or 200 mm above the floor unless set
  otherwise: the hoses, tubes and pipes of the schematic are routed along them like
  wires along cable ducts. The floor plan shows them dashed.

The check reports devices on a wall that are neither, rails or devices over a door or
window, pipe runs across a door or window, items on the ceiling outside the floor plan,
inner walls leaving the room, cabinets and floor-standing parts in the swing of a door
or in an inner wall, floor-standing parts on the place of a cabinet or not made to
stand on the floor, and cabinets and floor-standing parts outside the room (or across a
wall), cabinets higher than its ceiling, wall penetrations that lead into no room. Rooms
have no climate calculation and no NC data.

**Several rooms** stand at their places in the plant (*Rear left corner of the room*
in its properties, commands `add_room` / `set_room` with `x`, `z` and `angle`). The
floor plan of a room shows the rooms around it faintly with their names and cabinets;
a double-click opens one of them. *Layout space › Show › All enclosures* shows every
room and cabinet at its place in 3D, with the routes between them.

![Floor plan of the electrical_room example: L-shaped room, inner wall, door, window and the two cabinets](../images/room.png)

## 19-inch racks

The enclosure system *19-inch rack* (`rack19`, *Layout space › New enclosure*, command
`add_rack` or `add_enclosure` with `system` `rack19`) is a frame with a perforated door
and two pairs of 19-inch mounting rails (IEC 60297) instead of a mounting plate; its
standard sizes run from a 12 U wall rack to 47 U, and the rails are a part of the bill
of materials (`GEN.RK-RAIL-42U`). Its rack units (1 U = 44.45 mm) are numbered from the
bottom beside the rails in the front view and on the mounting layout page.

- **19-inch parts** are as wide as a 19-inch panel (482.6 mm) and a whole number of
  units high (technical data `Rack units: 2`): the catalog has a network switch, a
  patch panel, a PDU, a server, a UPS, blank panels, a shelf and a cable management
  panel. *Insert › 19-inch part* mounts one in the highest free units of the rack shown
  (command `mount_in_rack` with `unit`, counted from the bottom); placed or dragged,
  devices and parts snap onto whole units, centred, their front panel on the front
  rails, and are held by the rails (no holes in a plate).
- Units taken twice are collisions; the check reports 19-inch parts outside the units
  and, as a hint, other parts on the mounting level (they need a shelf or a mounting
  plate). Report *19-inch rack elevation* (and `list_racks`) lists the parts of every
  rack from the top with their units and depths, and the free units.
- Racks bay into line-ups and stand in rooms like other cabinets.

## 3D routing

Cables, tubes and (optionally) wires are routed through the line-up: in cable ducts,
along routing paths drawn over the mounting plate, or freely around the devices;
segregation of the classes power, control, signal and fluid, routing reservations, bend
radii, cable bundles with their branches, shield terminations (shield clamp rail,
pigtail) and their checks; routes can be frozen for production and stay as they are
until they become invalid.

Between rooms, a route passes the wall penetrations that join them, through as few
walls as possible: through the first room to a penetration, through the rooms between,
through the last room to its end (its legs per room and cabinet in the routing list);
its length includes the ways through the walls. Without such a penetration the route
stays unrouted (*no wall penetration leads from room to room*). A route from a device
of a room to the outside of the building ends at its nearest building entry; of several
wall penetrations joining the same two rooms the route takes the one with the shortest
way.

In a room, routes run through its free space: around its walls, inner walls, the
cabinets standing in it and the swing of its doors, preferring its cable ducts and pipe
runs. Running horizontally through the open space of a room, more than 300 mm from its
walls, its ceiling and what stands in it, costs ten times as much: pipes and cables
keep to the walls and the ceiling and drop straight to a device standing in the room.
Cables and pipes turn only after half their bend radius straight on (where there is
such a way); of the ways that cost about the same, a route takes one whose bends keep
their distance (a short stretch first or last). A route between a cabinet and a device in its
room (or two cabinets of one room) runs in legs: through the cabinet to a cable gland on
its roof, sides or rear (one of its own cabinet first, otherwise of the line-up; without
one through the middle of the roof, which is reported), then through the room. Frozen, such a route keeps its legs.
Hoses, tubes and pipes end at the nozzles of the generated models of their devices (the
suction and the pressure nozzle of a pump, the nozzles of a vessel, both ends of a valve
or a fitting, the outlets of a manifold): the route comes to a point twice as far out of
the nozzle as the pipe is thick (at least 30 mm) and goes on straight into it. A valve,
meter or check valve in a pipe is taken as mounted the way round that brings both of its
pipes nearer to where they come from.
The example `pump_room` shows floor-standing pumps, a pipe run and the routes.

In a schematic, the context menu of a placed device shows it in the panel layout (its
cabinet's mounting plate or door, the wall or floor plan of its room) or in the 3D view
with the camera close to it (*Show in panel layout*, *Show in 3D view*).

The routes are part of the project's analysis. When a project is opened its analysis is
built in the background: the editor shows a window with the step it is at (netlists,
connections, panel wiring, 3D routing with the route it is at, duct fill, checks) and
shows the project once it is ready.

The routed panel is the input of the [wiring and mounting assistants](workshop.md) and of
the 3D exports (glTF, STEP, IGES, IFC, JT, Parasolid; see
[output and data exchange](data-exchange.md)).
