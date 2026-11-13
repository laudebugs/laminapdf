# `outline-move-into-itself`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A move names the bookmark itself, or one of the bookmarks under it, as its new parent or as the place to put it next to.

## Why

A bookmark goes with its children when it moves, so it can't move into its own branch: the outline would become a loop.

## How to fix

Choose a parent outside the bookmark's branch. To move children up, move them one by one, or delete the bookmark with `children: promote`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
