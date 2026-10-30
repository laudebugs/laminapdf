# `invalid-color`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Color::hex` was given a string that is not `#rgb` or `#rrggbb`.

## Why

Hex colours are a `#` followed by three or six hexadecimal digits, in either case.

## How to fix

Write the colour as `#3366cc` or `#36c`, or use `Color::rgb`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `text`: the string as given (untrusted: caller text, never in the message).
