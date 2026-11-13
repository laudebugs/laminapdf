# `annotation-not-editable`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

The annotation can't be changed or deleted by the annotation ops. Either it is a widget, the part of a form field that shows on the page, or it is stored as a direct dictionary inside the page's `/Annots` array instead of as an object of its own.

## Why

Widgets belong to their form fields, and editing one on its own would leave the field inconsistent; the form ops edit them. ISO 32000-2 Table 31 asks for `/Annots` to hold references to annotation objects, and a direct dictionary has no object an edit could change in place.

## How to fix

Edit a widget through its form field. An annotation stored inside the page's list can't be edited yet: Lamina PDF reads it, shows it and keeps it as it is when saving.

## Example

```json
{ "op": "deleteAnnot", "args": { "id": "a14.0" } }
```

Object 14 is a text field's widget.

## Data fields

None. The `location` is the annotation's `object` and `id` when it has an object.

## For contributors

The decisions and todos behind this code:

- [E4b](../todos/done/E4b-annotation-model-and-overlay.md): the annotation model and the annotation ops' core
