# Resolution: an edited XMP packet failed to parse again

[Diagnosis](detail.md).

## The fix

The tag scanner (`crates/laminapdf-model/src/xmp/tree.rs`) requires every element and attribute name, `xmlns:` declarations included, to be an XML 1.0 `Name` (§2.3). A packet that names one otherwise is malformed, so `Packet::parse` refuses it with a syntax error, and the writer never meets a prefix XML can't hold.

## How it was checked

- The crash input is saved in `fixtures/regressions/xmp_parse/` and replays without a panic.
- `a_name_xml_does_not_allow_is_malformed` checks a `?dc` prefix, an attribute starting with a digit and an element starting with `-`.
- Every golden packet still parses and writes back unchanged (the model crate's 268 tests).
