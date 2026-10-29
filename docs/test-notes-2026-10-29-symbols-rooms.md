# Test session 2026-10-29: placing symbols, rooms with cabinets

Web build (`trunk serve`, dev profile) in a 1024 × 768 browser pane, English UI,
fresh project. Flows: symbols from the Insert center on a schematic page; panel
layout workspace with two enclosures and a room (floor plan, walls, door, window,
inner wall, rail, 3D view, checks, delete / undo).

## Status

Fixed in the same branch: bugs 1 to 4 and flaws 6 to 26 (see the commit). Bug 5, the
folded menu needing a second click, showed no cause in the code and is left for a
retest outside the hidden browser pane, which delays frames. Bug 4's cause was not
proven either: a press in a canvas now takes the keys back from any focused widget.

## Bugs

1. **Enclosure numbering counts rooms.** With `+CAB1` and `+ROOM1` in the layout,
   *New enclosure* proposes `+CAB3`. `new_enclosure_dialog` uses
   `enclosures.len() + 1` (`openecraft/src/panel_ui.rs`), while `new_room` searches
   the first free number.
2. **R turns a cabinet about its rear-left corner.** A cabinet just put into the room
   (at x 0, z 0) lands outside the room after one R (the check reports it). Turning
   should keep the footprint in place (turn about its centre or re-snap it inside).
3. **Wall summary counts openings as devices.** The room properties show
   "1 rails, 0 cable ducts, 1 devices" for a wall that carries only a window or a
   door (`items.len() - rails - ducts`). The counts are also always plural
   ("1 rails").
4. **Keys of the floor plan dead until Esc (seen once, not reproduced).** After
   *Put a cabinet into the room* and a drag of the cabinet, R did nothing three times
   (after a drag, after a click, after a tab click); after one Esc the same R worked
   and kept working. The key block in `plan.rs` and `panel_ui.rs` is guarded by
   `!egui_wants_keyboard_input()`, so a widget that keeps keyboard focus (the
   combobox or its popup is the suspect) silences R, Del, Enter and Backspace in the
   canvas without any feedback. A canvas click could surrender / take focus, or the
   guard could test `text_edit_focused()` instead.
5. **Collapsed ribbon group menu needs a second click (2 of 5 times).** With the
   window narrow enough to fold the *Draw* group, choosing *Door* / *Window* and then
   clicking the group button again sometimes left the menu closed; the click then went
   through to the canvas ("Click on a wall").

## UX and workflow flaws

### Narrow window (1024 px) and clipping

6. Texts are clipped instead of wrapped or truncated with a tooltip: the left pane
   title "Enclosures and rooms" and its hints, the room row (its delete icon is off
   the pane while the cabinet rows show theirs), the cabinet properties row *Place in
   the room* (x, z and angle unreachable), the wall summaries, Insert center entries
   (letter badge and star cut), message texts (no full text in the tooltip either).
7. The hint lines drawn into the floor plan ("Drag a wall to move it … Del re…",
   "Click the corners of the walls … Backspace rem…") overflow the view.
8. The *New enclosure* dialog opens at the top-left corner over the ribbon instead of
   over the layout view.
9. The Insert center pane is about a third of the right column; the tree needs a lot
   of scrolling, and the floor plan re-fits on every re-entry but not when the pane is
   resized with the splitter (part of the room leaves the view).

### Floor plan

10. The current wall is drawn magenta (the face shown in the wall view) and the
    dragged / hovered one orange, without a legend or tooltip.
11. A cabinet outside the room, across a wall or in a door swing looks like any
    other; only the warning count changes. The panel workspace has no Messages pane
    by default, so the count in the status bar is the only sign.
12. The pointer readout shows raw millimetres (x 4001, z 2967) while corners snap to
    50 mm; the snapped value would be more useful.
13. The "Cabinets in the room" list shows x and z but not the angle; the cabinet's own
    properties hold the angle but the row is clipped (see 6).
14. Inner walls snap to the 50 mm grid only, not to the room walls: a click just
    inside the rear wall leaves a 50 mm gap (1.45 m instead of 1.50 m) with no hint.
    After Esc ends the chain the tool stays active with the same hint; a second Esc
    is needed to get back to Select.
15. A window is drawn in the plan as a thin "glass" line, hardly visible at fit zoom.
16. Door and window properties show the generic item rows (Part "-", Collision class
    Automatic, Horizontal / Vertical position Free, X/Y in wall coordinates) that mean
    little for an opening.
17. With the *Draw* group folded, nothing in the ribbon shows which tool is active;
    only the status bar text does.
18. Deleting a room that holds two cabinets, a rail, a door and a window asks no
    confirmation (undo restores everything).

### Views and messages

19. The 3D view of a room shows the enclosures' *Close door* button.
20. Dimension order differs: the list says "room 4000 × 3000 × 2800" (w × d × h), the
    status bar "+ROOM1 4000 × 2800 × 3000 mm" (w × h × d).
21. In the wall view the dashed edge of the wall rectangle moved from the floor to the
    ceiling while the rail tool was active; the meaning of the dashed edge is not
    explained anywhere.
22. Opening Messages from the status bar in the panel workspace puts it into the
    centre tab group, hiding the layout view; *Click to show* on a layout message
    selects the cabinet but does not bring the *Panel layout* tab to the front.
23. *Insert › Wall mounted part* in a fresh project says "No parts marked for wall
    mounting." and stops; no pointer to the part editor (Mounting data › Wall
    mounting) or the catalog filter, and the built-in catalog ships no such part.
24. "Devices to place: All devices with mounting data are placed." for a project
    whose devices have no parts yet reads as if the work were done.

### Schematic

25. At 35 % page zoom the symbol being placed is only a crosshair; the ghost becomes
    visible after zooming in.
26. The contact image beside a coil turned by 90° overlaps its A1 label (minor, seen
    at 123 %).

## Works as expected

- Insert center: click an entry, ghost at the pointer, status hint, placing continues,
  R / M on the ghost, Esc ends; tags -K1 … -K3 assigned; recently used filled.
- Contact placed with the coil selected gets -K3 and 13/14, cross references live,
  contact image at the coil.
- Search "lamp" finds IEC, GB and GOST lamps; auto-connect drew the wire from the
  contact to the lamp ("1 connection drawn automatically"); undo removes lamp and wire
  in one step, redo restores them.
- Enclosure and room creation, *Put a cabinet into the room*, dragging cabinets
  (10 mm snap, live x/z), dragging a wall ("Wall 2: moved by 560 mm"), R turns (once
  keys work), drawing an L-shaped outline, *Rectangular* back to 4 walls keeping door,
  window and inner wall, door with swing, window 900 mm above the floor, inner wall
  chain, rail on a wall with its part chooser, double-click opens a cabinet or a wall
  view, 3D view with walls, openings and cabinets, checks "stands in the swing of the
  door" / outside the room, delete room and undo.

## Not reproducible or tooling

- The browser automation's wheel zooms the canvases but does not scroll egui scroll
  areas (dragging the scrollbar does), so whether the Layout properties pane scrolls
  to *Layout checks* at 768 px height could not be verified; the Insert center
  scrolls fine by its bar.
- Wheel zoom anchoring on the page could not be judged (one tick gave no zoom, five
  ticks jumped).
