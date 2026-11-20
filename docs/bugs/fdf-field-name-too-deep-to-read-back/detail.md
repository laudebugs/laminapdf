# A field with a long dotted name in FDF didn't read back

[Resolution](resolution.md).

## What we saw

The nightly `fdf` fuzz target failed (2026-11-20, run `44729481527`, job "Fuzz fdf"): `the fields written didn't all read back, left: 0, right: 1`. The target reads an FDF file, writes what it read as FDF, and reads that again; the second read found no fields where the first found one.

## How to reproduce

Run the crash input (`crash-40def8bf…` in the run's `fuzz-artifacts-fdf`), saved as `fixtures/regressions/fdf/81ae8623085f634f2a22e189338aadb9dbc26f41`, minimised, through `laminapdf-interchange`'s `tests/regressions.rs`. A smaller file shows the same thing: one field whose `/T` is a hundred periods (four shown here).

```
%FDF-1.2
1 0 obj
<< /FDF << /Fields [<< /T (....) /V (1) >>] >> >>
endobj
trailer
<< /Root 1 0 R >>
```

## Cause

The fuzzer wrote a `/T` that ran on into the rest of the file, so the field's name held more than 64 periods, which makes about a hundred parts. ISO 32000-2 §12.7.4.2 says a partial name shall not contain a period, but the reader is tolerant and took the whole string as the field's fully qualified name: one field with a hundred or so parts.

The writer is strict: it writes no period inside a `/T`. It nests each part of a name one dictionary deeper, under `/Kids` (§12.7.8.3.2, Table 246), so this field came out a hundred levels deep. The reader, though, goes down at most `Limits::max_tree_depth` levels (64) and leaves out anything deeper. So its own output lost the field.

The two sides counted depth in different units. The reader counted dictionaries; the writer's depth is the number of parts in the name. The limit has to count what the writer nests.

XFDF has the same gap. Its writer also nests a `<field>` per part, and the XML reader refuses a document more than 64 elements deep, so an XFDF field named with 62 or more parts read once and failed to read back (`interchange-too-large`). The fuzzer hadn't found it yet.
