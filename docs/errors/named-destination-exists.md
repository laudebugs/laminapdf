# `named-destination-exists`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A rename gives a destination a name that another destination of the file already has.

## Why

Each name points at one place, so two destinations can't share it. Renaming over it would lose the other destination.

## How to fix

Choose another name, or delete the other destination first if it is no longer needed.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/done/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
