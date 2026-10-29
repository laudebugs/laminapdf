# `collection-index`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Font::from_collection` was asked for a member past the end of the font collection.

## Why

Members are numbered from 0; a single font counts as a collection of one.

## How to fix

Use an index below `Font::collection_len(bytes)`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `index`: the member that was asked for.
- `count`: how many members the collection has.
