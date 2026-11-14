# `dictionary-key-skipped`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A dictionary had something other than a name where a key belongs. Lamina PDF skipped it and read the rest of the dictionary, as pdf.js and PDFium do.

## Why

Damage inside the file: bytes overwritten or spliced in, so a stray word, number, string or unbalanced `)` or `>` sits between a dictionary's entries.

## How to fix

Nothing, if the document reads correctly: re-saving writes the dictionary without the stray token. In strict mode the warning is an error, so the file doesn't open.

## Example

```text
4 0 obj
<< /Type /Page /Parent 2 0 R ) /MediaBox [0 0 612 792] >>
endobj
```

The `)` can't be a key. Lamina PDF skips it, keeps `/MediaBox`, and raises the warning with the page's `object`. A word such as `Size 6` in place of `/Size 6` is skipped the same way, one token at a time; an array or dictionary is skipped whole. An object keyword such as `endobj` still ends the dictionary with a syntax error, because the dictionary was never closed.

## Data fields

None.

## For contributors

- [ADR 0003](../decisions/0003-reader-tolerance-and-limits.md): the tolerance rule and why every repair is a warning
