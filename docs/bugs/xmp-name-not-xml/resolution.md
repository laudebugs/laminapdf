# Resolution: an edited XMP packet failed to parse again

[Diagnosis](detail.md).

## The fix

The tag scanner (`crates/laminapdf-model/src/xmp/tree.rs`) requires every element and attribute name to be an XML 1.0 `Name` (§2.3), and every declared prefix to be one too, as Namespaces in XML requires (an NCName). `xmlns:1dc` is a `Name` as a whole, but its prefix `1dc` is not, and `<1dc:title>` would fail the same way. A packet that breaks either rule is malformed, so `Packet::parse` refuses it with a syntax error, and the writer only meets prefixes a tag can start with.

## How it was checked

- The crash input is saved in `fixtures/regressions/xmp_parse/` and replays without a panic.
- `names_xml_does_not_allow_are_malformed` checks the prefixes `?dc`, `1dc`, `-dc` and `.dc`, an attribute starting with a digit and an element starting with `-`.
- A probe of the fuzz target's edit on each prefix: the four are refused, and `d·c`, a legal one, writes and reads back.
- No tag or attribute name in the 40 XMP packets among the fixtures breaks the rule.
- Every golden packet still parses and writes back unchanged (the model crate's 268 tests).
