# An edited XMP packet failed to parse again

[Resolution](resolution.md).

## What we saw

The full lane's fuzz smoke run on 2026-11-11 (commit `1ad0ce12`) failed `xmp_parse`: "the edited packet doesn't parse: the packet is not well-formed XML near byte 4962". The target parses a packet, sets its title, authors, date and keywords, writes it, and parses the result again.

## How to reproduce

Replay the saved input, `fixtures/regressions/xmp_parse/b282388ff548398231876d162bc5c25d1f0a68f2`, with `cargo nextest run -p laminapdf-model --test regressions`.

## Cause

The input declares Dublin Core under the prefix `?dc`: `xmlns:?dc='http://purl.org/dc/elements/1.1/'`. `?dc` is not an XML name, but the parser checked tag and attribute names only for colons and white space, so it accepted the declaration. Writing the new title, the writer used the prefix bound to Dublin Core and wrote `<?dc:title>…</?dc:title>`. XML reads `<?dc:title>` as a processing instruction, so the closing tag matched nothing, and the parse failed at the packet's end.
