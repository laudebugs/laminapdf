# `xfa-removed`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

Flattening a form removed its XFA part while other fields were left. The fields that are left now work as a plain PDF form, in every PDF app.

## Why

Some forms carry their fields twice: as PDF form fields, and in an XFA part that Adobe's apps show instead. Flattening draws the chosen fields into their pages and removes them from the PDF form, but it can't remove them from the XFA part. Left in place, the XFA part would make Adobe's apps show those fields again, empty and editable, over the answers drawn on the page. So Lamina PDF removes it.

## How to fix

Nothing: the flattened answers are on the page, and the fields that are left keep their values. Flatten every field at once if you want no form left at all. The warning never blocks the edit or the save.

## Example

Flattening the `name` field of a form with an `/XFA` entry and two other text fields.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E8p](../todos/done/E8p-flatten-form.md): flattening forms
- [E8 plan §2.1](../plans/editor/e8-form-filling.md#21-ops): what `FlattenForm` removes
