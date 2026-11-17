# `flatten-no-appearance`

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit flattened annotations, and some of them stayed in place: they have no appearance, and Lamina PDF can't draw one for their kind. The others were flattened.

## Why

Flattening draws each annotation's appearance, the picture a viewer shows for it, into the page. Lamina PDF draws one for notes, carets, text markup, ink, shapes, lines, polygons and text boxes that lack it. For other kinds, such as a file attachment or a sound, each viewer draws its own icon, so there is no picture to keep.

## How to fix

Nothing, if the annotations should stay. Otherwise open the file in the app that made them, which may give them an appearance, or delete them. The warning never blocks the edit or the save.

## Example

Flattening a page with a file attachment that has no `/AP` entry.

## Data fields

- `annotations`: the IDs of the annotations left in place.

## For contributors

The decisions and todos behind this code:

- [E5e](../todos/done/E5e-flatten-and-summary.md): flattening annotations
- [E5 plan §2.1](../plans/editor/e5-review-stamps-interchange.md#21-ops): what flattening leaves in place
