# `invalid-feature`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An OpenType feature setting given to the shaper (`"liga=0".parse::<Feature>()`) did not parse.

## Why

Feature settings use HarfBuzz's syntax without ranges: `tag` or `+tag` (on), `-tag` (off), `tag=N`, `tag=on` or `tag=off`, where the tag is one to four printable ASCII characters. A sign together with a value (`+liga=1`), an empty or longer tag, or a value that is not a non-negative integer is refused.

## How to fix

Write the setting as `liga=0`, `-liga`, `smcp` or `aalt=2`, or build it with `Feature::new(Tag::new(b"liga"), 0)`.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `feature`: the setting as given (**untrusted**: caller text).
