# Revisions, review and change management

Revisions with locked pages and marked changes, the approval workflow, baselines with compare and merge, comments and redlining, and change requests.

## Revisions

A revision stores a snapshot of all pages and can release (lock) them;
locked pages are read only. Changes since the last released revision (added / changed /
removed symbols, wires, texts, pages) are listed, marked on the pages with a revision
triangle and reported in the *revision overview*; changed items name the changed
properties with old and new value (`Device tag: -K1 › -K2; Position`).

## Release workflow

With approvers defined (*Project data › Revisions… › Approval
workflow*: user names and the number of approvals needed) a new revision starts as a
draft, is submitted for review, approved or rejected by the approvers (recorded with
user, date and note) and released once approved. Pages changed after the submission
block the release until the revision is submitted again (earlier approvals become void).

## Baselines and compare / merge

A baseline stores the whole project (pages, devices,
parts, cables, panel layout, settings); releasing a revision stores one automatically.
*Project data › Compare and baselines…* compares the project with a baseline, the pages of a
revision or another project file: added / removed / changed pages (with the changed
items), devices, parts, potentials, cables, harnesses, enclosures, terminal strips,
plugs, PLC cards and settings sections. Selected differences are taken over.

## Review: comments and redlining

Comments on a page position with status (open, in
progress, resolved, rejected), author, date and replies; redline (to be changed) and
greenline (changed as built) markup as line, rectangle, revision cloud or text. They are
drawn on a layer of their own (shown on screen, not in exports), listed in the *Review*
tab of the navigator and can be added to released pages and to pages another user has
locked. On the project server they are merged record by record without locks.
**PDF comments** a reviewer made in a PDF export (Acrobat or another viewer: notes,
text boxes, highlights, shapes, replies) are imported into the review layer
(*Review › Import PDF comments…*, command `import_pdf_comments`); rectangles and
lines also become redline markup, comments imported before are skipped.

## Change requests

Requests from inside or outside the project (number, title,
description, requester, origin, priority, status, affected pages and devices,
implementing revision) with linked comments (*Project data › Change requests…*). Exchange
with other systems as CSV (`Number;Title;Description;Requester;Origin;Date;Priority;
Status;Pages;Devices;Revision`) or JSON; importing updates requests with a known number.
