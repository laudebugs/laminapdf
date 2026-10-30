# `unknown-destination`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A link or outline item of a new document targets `LinkTarget::Named(name)`, but `Builder::destination` never defined that name.

## Why

A named destination is looked up in the document's `/Names /Dests` tree (ISO 32000-2 §12.3.2.4); a name that isn't there goes nowhere in every viewer.

## How to fix

Define the name with `Builder::destination(name, page_index, fit)` before `Builder::finish`, or link to the page directly with `LinkTarget::Page`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `name`: the name as given (untrusted: caller text, never in the message).
