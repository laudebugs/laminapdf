# `pdfa-info-mismatch`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The file says it conforms to PDF/A, which requires every entry of the Info dictionary to have an equal property in the XMP packet (ISO 19005-2 §6.6.2.3.1), and some of them differ. Saving it as it is writes a file that no longer meets its PDF/A level.

## Why

An edit wrote only one of the two places, for example a tool that changed `/Title` in the Info dictionary and left the XMP title alone. Lamina PDF's metadata edits write both, so this usually comes from the file as it was.

## How to fix

Set the fields again with `setMetadata`, which writes Info and XMP together, or remove the fields that differ from both copies with `removeMetadata`. The warning never blocks a save.

## Example

A PDF/A-2b file whose Info `/Title` is "Draft" while its `dc:title` is "Final": `doc.plan()` lists `pdfa-info-mismatch` with `keys` `["title"]`.

## Data fields

- `keys`: the fields whose Info entry has no equal XMP property, by their metadata names (`title`, `authors`, `subject`, `keywords`, `creator`, `producer`, `created`, `modified`).

## For contributors

The decisions and todos behind this code:

- [E1c](../todos/done/E1c-metadata-ops.md): the conformance warnings
