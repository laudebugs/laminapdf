# `xobject-cycle`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A Form XObject draws itself, directly or through other forms; the inner `Do` was skipped.

## Why

A malformed or hostile file: forms that draw each other without end would never finish.

## How to fix

Nothing to do.

## Example

```text
8 0 obj << /Subtype /Form /Resources << /XObject << /Me 8 0 R >> >> >> stream /Me Do endstream
```

The form draws itself: the inner `/Me Do` is skipped once lamina sees form 8 a second time.

## Data fields

- `object`: the form reached again.

The `location` is `object` (the form reached again).

## For contributors

The decisions and todos behind this code:

- [ADR 0023](../decisions/0023-content-stream-reading.md)
