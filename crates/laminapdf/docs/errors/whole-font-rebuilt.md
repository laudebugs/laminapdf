---
title: "whole-font-rebuilt"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/whole-font-rebuilt/"
---

# whole-font-rebuilt

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

A font whose `OS/2.fsType` forbids subsetting was embedded with every glyph, but rebuilt rather than byte for byte.

## Why

It is a variable-font instance or has CFF2 outlines; PDF can embed neither as it is, so `subsetter` instanced it (or converted CFF2 to TrueType outlines) while keeping every glyph. Its `cmap` and layout tables are dropped.

## How to fix

Check the font's licence allows this, or use a static TrueType/CFF version of the font.

## Example

Registering a variable font instance (`font.instance(&[(OpenTypeTag::WEIGHT, 700.0)])`) whose `fsType` forbids subsetting: the font is embedded with every glyph, but instanced, so not byte for byte.

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
