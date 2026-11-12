# E1b — XMP packet parse and update for Info-mapped properties

| Field | Value |
| --- | --- |
| ID | `E1b` |
| Status | `done` |
| Plan | [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B5c](./B5c-lamina-facade-and-examples.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E1a](./E1a-editing-foundation-feature-plan.md), [X3a](./X3a-nightly-lane.md) |
| Dependency notes | [E1a](../done/E1a-editing-foundation-feature-plan.md), `B5` ([pdf-library §B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate): builder and `laminapdf` crate) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e1b-xmp-packet-model` |
| PR title | `feat(model): parse and update the XMP metadata packet` |
| Size | L (600–1,500) |
| PR | [#398](https://github.com/laudebugs/laminapdf/pull/398) |

## Goal

Give `laminapdf-model` a typed view over the catalog's `/Metadata` XMP packet that can read and rewrite the properties mapped to the Info dictionary (`dc:title`, `dc:creator`, `dc:description`, `pdf:Keywords`, `pdf:Producer`, `xmp:CreatorTool`, `xmp:CreateDate`, `xmp:ModifyDate`, `xmp:MetadataDate`, `xmpMM:DocumentID`/`InstanceID`) while preserving everything else byte-for-byte where possible. The metadata ops (E1c) and the general XMP editor (E3e) build on it.

## Context

- [04 §5 Metadata](../../research/pdf/04-interactive-security.md#5-metadata): Info dictionary keys, XMP schemas and how they map
- [library B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate): the Info + XMP writer used for generation, to be shared, not duplicated
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): `pdfaid:`/`pdfuaid:` identification schemas written for generated files
- [ADR 0005](../../decisions/0005-document-model.md) §4: views are thin, lazy and tolerant per field ([ADR 0003](../../decisions/0003-reader-tolerance-and-limits.md))
- ISO 16684-1 (XMP) and ISO 32000-2 §14.3.2 (metadata streams), §14.3.3 (Info: deprecated in 2.0 except dates)

## Scope

**In scope**

- `laminapdf_model::xmp` module: tolerant RDF/XML packet reader (xpacket wrapper, `rdf:Description` attributes and elements, `rdf:Alt`/`Seq`/`Bag`, `xml:lang`)
- `xmp::Packet` typed accessors for the Info-mapped properties and the identification schemas (`pdfaid:part`/`conformance`, `pdfuaid:part`) read-only
- A writer that updates only changed properties and keeps unknown schemas, extension schemas (`pdfaSchema:`), comments and padding; regenerates the packet canonically only when the input cannot be edited in place
- Creating a minimal packet for files without `/Metadata`
- Info ↔ XMP mapping table shared with `laminapdf-build` (B5) in one module

**Out of scope** (and which todo covers it)

- Metadata ops, save-time `MetadataDate`/`InstanceID` and PDF/A checks → [E1c](../done/E1c-metadata-ops.md)
- Editing arbitrary/custom/extension schemas in the UI (J.04 write) → [E3e](../done/E3e-xmp-editor.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `J.04` manage XMP
- **Crates and packages:** `laminapdf-model` (new `xmp` module), shared mapping with `laminapdf-build`
- **`Op` variants:** none (views and a writer used by ops in E1c)
- **`SaveConstraint`:** read-only here; the writer's output becomes a modified stream object in the overlay when an op uses it
- **Web vs desktop:** identical (core only)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `J.04` → `read` (core, pdf) (parse and typed read of the packet; `write`/`round-trip` comes with [E3e](../done/E3e-xmp-editor.md))

## Tasks

- [x] Choose the XML approach (a small tolerant reader over `quick-xml` or similar; check licence with `cargo deny` and measure wasm size delta) and record the choice in the E1 feature plan
- [x] Implement packet discovery: catalog `/Metadata` stream, decode filters, strip/keep the `<?xpacket … ?>` wrapper and trailing padding
- [x] Parse into an ordered tree that keeps unknown elements, attributes, namespaces and comments for lossless re-serialisation
- [x] Typed getters/setters for the Info-mapped properties, with language alternatives (`x-default`) and date conversion between PDF dates (`D:YYYYMMDDHHmmSSOHH'mm`, ISO 32000-1 §7.9.4: no apostrophe after the minutes) and ISO 8601; the reader tolerates the trailing `'` that older producers write, the writer never emits it
- [x] Writer: update in place keeping padding when the new packet fits; otherwise rewrite with fresh padding (2 KB) and `begin="\uFEFF"`
- [x] `xmp::Packet::new_minimal()` (`Packet`, since the disclosure scan's `XmpPacket` has the name) for files without metadata; namespace prefixes per ISO 16684-1
- [x] Move the B5 builder's Info ↔ XMP mapping into one shared table used by both generation and editing. The table (`MAPPINGS`), `property_element`, `escape` and the date conversion live in `laminapdf_model::xmp`; the builder writes each property with literal tags, which a unit test holds to what `property_element` writes for that row, since building them from the table at run time went over the text module's WebAssembly budget
- [x] Tolerance: malformed packets produce a warning code (ADR 0003) and are treated as absent for writing only after the UI confirms (exposed as a flag to E1c)

## Acceptance criteria

- [x] Parsing then writing an unchanged packet is byte-identical for every packet in `fixtures/corpus` that parses
- [x] Changing `dc:title` on fixtures from Acrobat, Word, LibreOffice, pdfTeX and Ghostscript keeps all other properties and schemas (XML-level comparison test); the packets are hand-written models of each producer's output, since no committed file has a real one (`fixtures/README.md`)
- [ ] A file with no `/Metadata` gets a packet that `exiftool` and veraPDF's XMP parser accept: veraPDF 1.30.3 accepts it (only 6.6.4-1, the missing PDF/A claim, fails when it replaces a PDF/A-2b example's packet) and passes every PDF/A and PDF/UA example whose packet the writer edited; `exiftool` wasn't installed, so it is unchecked
- [x] Malformed packets (fuzz corpus) never panic; a fuzz target runs 10 min in CI (`xmp_parse`, in the nightly's matrix)
- [ ] wasm size delta recorded in the PR description and within the B6 budget: recorded in the E1 plan §4 (base +420 B gzip, text +37 B, extract −152 B, measured locally against the same build without E1b); the text module is within budget only by CI's margin (main is 79 B over its budget in a local macOS build)
- [x] Date round-trip tests: `D:20261028143000+01'00` and the legacy `D:20261028143000+01'00'` both parse to the same instant; the writer emits only the form without the trailing apostrophe

## Post-merge checks

None.

## Tests

- Unit tests in `crates/laminapdf-model/src/xmp/` for every RDF form (attribute, element, `rdf:Alt` with languages, `rdf:Seq`)
- Golden packets in `fixtures/golden/xmp/` (input → edited output) — never rewritten by a test
- Corpus test: round-trip of every packet in `fixtures/corpus`
- Differential: values read vs `exiftool -j` on the corpus (nightly lane): not built here (`exiftool` couldn't be run); moved to [E1c](../done/E1c-metadata-ops.md) as a task beside its pypdf metadata differential
- Fuzz target `fuzz/fuzz_targets/xmp_parse.rs`

## Docs to update

- rustdoc for `laminapdf_model::xmp`
- `fixtures/README.md` provenance for new XMP fixtures
- [full-editor §E1](../../plans/full-editor.md#e1-editing-foundation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- XML parsing in core must stay bytes-in/bytes-out and wasm-friendly; no entity expansion (XXE/billion laughs) — reject DTDs
- Lossless editing of hand-written packets is hard; fall back to canonical rewrite and say so in the warning

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (git workflow §2's current rule)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
