# `empty-destination-name`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op names a destination with an empty name.

## Why

Links and bookmarks find a named destination by its name, so it needs at least one character.

## How to fix

Give the destination a name. Names are case-sensitive and can hold any text.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E3b](../todos/ongoing/E3b-bookmarks-and-destinations.md): the bookmark editor and named destinations
