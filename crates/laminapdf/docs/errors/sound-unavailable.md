---
title: "sound-unavailable"
version: "0.7.0"
url: "https://docs.laminapdf.com/reference/codes/sound-unavailable/"
---

# sound-unavailable

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A sound comment was asked for as a WAV file, and Lamina PDF couldn't make one. The annotation isn't a Sound annotation, it has no sound stream, or its samples are in a form Lamina PDF doesn't play.

## Why

A Sound annotation holds its recording as samples with a rate, a number of channels, a sample size and an encoding (ISO 32000-2 §13.3). Lamina PDF turns raw, signed, μ-law and A-law samples into WAV, which every browser plays, when their format is one PDF allows. Samples stored with a compression (`/CO`), and formats such as no channels or a rate of zero, are the exceptions.

## How to fix

Ask only for annotations whose record has the `Sound` subtype and a playable format. A record's sound details show its format, and say when it has no sound stream.

## Example

`sound_as_wav` given the ID of a highlight, or of a sound annotation whose samples carry `/CO /DPCM`.

## Data fields

None.
