# `interchange-unknown-format`

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An import was given bytes that are neither an XFDF file nor an FDF file, so nothing was imported. A form data import also takes Lamina PDF's form data JSON.

## Why

Lamina PDF tells the formats apart by their first bytes: an FDF file starts with `%FDF-`, an XFDF file with XML markup, and form data JSON with `{`. A PDF, a Word file or an empty file is none of them.

## How to fix

Pass the `.xfdf` or `.fdf` file that a PDF app exported. Acrobat and most PDF apps export comments as XFDF from their comments list, and form data from their forms tools. For `fill_form` (`fillForm`), pass an object with `fields`.

## Example

Importing a PDF instead of the comments exported from it.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E5d](../todos/done/E5d-xfdf-fdf.md): importing and exporting comments
- [E8g](../todos/done/E8g-form-data-and-flatten.md): importing and exporting form data
- [E5 plan §2.3](../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary)
