# `content-resource-missing`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An operator named a resource (font, XObject, graphics state, colour space, shading, property list) that is missing, of the wrong type or unreadable, or a Form XObject's content could not be decoded; the operator was skipped or ran without it (ADR 0023).

## Why

Resources lost in editing or merging, or damaged objects.

## How to fix

Check the page's `/Resources`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `offset`: the operator's byte offset in the decoded content.
- `category`: the resource category, e.g. `Font` (lamina's own vocabulary).

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
