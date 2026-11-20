# Resolution: a field with a long dotted name in FDF didn't read back

[Diagnosis](detail.md).

## The fix

Both readers now limit a field by the parts of its fully qualified name, the depth their writers nest it at. `fields::nests_within` counts the parts, and a field with more than the limit is left out with the fields below it, however the file nested it:

- **FDF** keeps names of up to `Limits::max_tree_depth` parts (64 by default). Each level of `/Kids` adds a part, so the same count still bounds how far the reader recurses.
- **XFDF** keeps names of up to 61 parts: the XML reader opens at most 64 elements, and a written field sits inside `<xfdf>` and `<fields>` with a `<value>` inside it.

A name the reader keeps now reads back from what the writer writes. The fuzz property stays as it was.

## How it was checked

- `fixtures/regressions/fdf/81ae8623085f634f2a22e189338aadb9dbc26f41` is the nightly's crash input, minimised. `every_saved_crash_input_runs_clean` in `crates/laminapdf-interchange/tests/regressions.rs` fails on it without the fix.
- With the readers put back as they were, both tests fail: the crash input panics on `the fields written didn't all read back` (`left: 0, right: 1`), the same message as the nightly. With the fix they pass.
- `field_names_deeper_than_the_writer_can_nest_are_left_out`, in the same file, reads a field with a hundred-part name in both formats, and checks that a name at the limit is kept and reads back.

## Left for later

The FDF limit assumes the parser's nesting limit (`Limits::max_nesting_depth`, 256) leaves room for twice `max_tree_depth` levels, which the defaults do. A caller who raises `max_tree_depth` past about 126 without raising the nesting limit could still write a field it can't read back.
