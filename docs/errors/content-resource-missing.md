# `content-resource-missing`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An operator named a resource (font, XObject, graphics state, colour space, shading, property list) that is missing, of the wrong type or unreadable, or a Form XObject's content could not be decoded; the operator was skipped or ran without it.

## Why

Resources lost in editing or merging, or damaged objects.

## How to fix

Check the page's `/Resources`.

## Example

```text
/Im7 Do      % /Resources << /XObject << /Im1 8 0 R >> >>
```

`/Im7` is not in the page's resources, so the `Do` draws nothing. `data.category` is `XObject`; the resource's name, which comes from the file, is never put in the message.

## Data fields

- `offset`: the operator's byte offset in the decoded content.
- `category`: the resource category, e.g. `Font` (Lamina PDF's own vocabulary).

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.

## For contributors

The decisions and todos behind this code:

- [ADR 0023](../decisions/0023-content-stream-reading.md)
