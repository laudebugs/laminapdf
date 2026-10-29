# Error and warning codes

One page per code in [`fixtures/codes.json`](../../fixtures/codes.json), as [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §6 requires. Every `docsUrl` points here. A PR that adds a code adds its entry to `codes.json` (its `kind`, `summary` and `hint`), the arm in its type's `kind()` (an exhaustive match, checked against `codes.json` by that crate's tests), and its page here, with the headings *What happened*, *Why*, *How to fix*, *Example* and *Data fields* and a `- Kind:` line matching `codes.json`. `crates/lamina-cos/tests/codes.rs` checks the entries and pages, and that every page here is for a listed code. [L1b](../todos/ready/L1b-error-contract-codegen.md) expands the pages and generates the tables.

## Warnings

| Code | Kind |
| --- | --- |
| [`missing-header`](missing-header.md) | `input` |
| [`xref-repaired`](xref-repaired.md) | `input` |
| [`xrefstm-ignored`](xrefstm-ignored.md) | `input` |
| [`stream-length-recovered`](stream-length-recovered.md) | `input` |
| [`stale-generation`](stale-generation.md) | `input` |
| [`page-tree-node-skipped`](page-tree-node-skipped.md) | `input` |
| [`page-count-mismatch`](page-count-mismatch.md) | `input` |
| [`flate-truncated`](flate-truncated.md) | `input` |
| [`objstm-index-mismatch`](objstm-index-mismatch.md) | `input` |
| [`object-number-out-of-range`](object-number-out-of-range.md) | `limit` |
| [`aes-padding-invalid`](aes-padding-invalid.md) | `security` |
| [`perms-mismatch`](perms-mismatch.md) | `security` |
| [`warnings-truncated`](warnings-truncated.md) | `limit` |
| [`invalid-box`](invalid-box.md) | `input` |
| [`invalid-rotate`](invalid-rotate.md) | `input` |
| [`name-tree-node-skipped`](name-tree-node-skipped.md) | `input` |
| [`outline-cycle`](outline-cycle.md) | `input` |
| [`outline-item-skipped`](outline-item-skipped.md) | `input` |
| [`invalid-date`](invalid-date.md) | `input` |
| [`invalid-page-label`](invalid-page-label.md) | `input` |
| [`invalid-entry`](invalid-entry.md) | `input` |
| [`content-unknown-operator`](content-unknown-operator.md) | `input` |
| [`content-bad-operands`](content-bad-operands.md) | `input` |
| [`content-unbalanced-q`](content-unbalanced-q.md) | `input` |
| [`content-unbalanced-bt`](content-unbalanced-bt.md) | `input` |
| [`content-unbalanced-marked`](content-unbalanced-marked.md) | `input` |
| [`content-syntax`](content-syntax.md) | `input` |
| [`content-resource-missing`](content-resource-missing.md) | `input` |
| [`inline-image-unterminated`](inline-image-unterminated.md) | `input` |
| [`xobject-cycle`](xobject-cycle.md) | `input` |
| [`content-path-unpainted`](content-path-unpainted.md) | `input` |

## Errors

| Code | Kind |
| --- | --- |
| [`not-pdf`](not-pdf.md) | `input` |
| [`missing-root`](missing-root.md) | `input` |
| [`reference-cycle`](reference-cycle.md) | `input` |
| [`limit-exceeded`](limit-exceeded.md) | `limit` |
| [`strict`](strict.md) | `input` |
| [`data-pending`](data-pending.md) | `state` |
| [`source`](source.md) | `io` |
| [`type`](type.md) | `input` |
| [`invalid-structure`](invalid-structure.md) | `input` |
| [`unset-object`](unset-object.md) | `internal` |
| [`repaired-base`](repaired-base.md) | `state` |
| [`encrypted-base`](encrypted-base.md) | `unsupported` |
| [`password-required`](password-required.md) | `security` |
| [`wrong-password`](wrong-password.md) | `security` |
| [`unsupported-security-handler`](unsupported-security-handler.md) | `unsupported` |
| [`unexpected-eof`](unexpected-eof.md) | `input` |
| [`syntax`](syntax.md) | `input` |
| [`unsupported-filter`](unsupported-filter.md) | `unsupported` |
| [`decompress`](decompress.md) | `input` |

## Op errors

| Code | Kind |
| --- | --- |
| [`nested-transaction`](nested-transaction.md) | `state` |
| [`no-transaction`](no-transaction.md) | `state` |
| [`unknown-page`](unknown-page.md) | `usage` |
| [`direct-page`](direct-page.md) | `unsupported` |
| [`unsupported-version`](unsupported-version.md) | `unsupported` |
| [`invalid-op-log`](invalid-op-log.md) | `input` |
| [`saved-not-rebased`](saved-not-rebased.md) | `state` |
| [`no-pending-save`](no-pending-save.md) | `state` |

## Save errors

| Code | Kind |
| --- | --- |
| [`sink`](sink.md) | `io` |
| [`not-rebased`](not-rebased.md) | `state` |
| [`sink-not-empty`](sink-not-empty.md) | `usage` |
| [`would-invalidate-signatures`](would-invalidate-signatures.md) | `security` |
| [`requires-rewrite`](requires-rewrite.md) | `state` |

## Text errors (`lamina-text`)

| Code | Kind |
| --- | --- |
| [`invalid-font`](invalid-font.md) | `input` |
| [`collection-index`](collection-index.md) | `usage` |
| [`unknown-axis`](unknown-axis.md) | `usage` |
| [`font-not-embeddable`](font-not-embeddable.md) | `usage` |
| [`missing-glyph`](missing-glyph.md) | `input` |
| [`glyph-out-of-range`](glyph-out-of-range.md) | `usage` |
| [`unknown-font`](unknown-font.md) | `usage` |
| [`subset-failed`](subset-failed.md) | `unsupported` |
| [`invalid-axis-value`](invalid-axis-value.md) | `usage` |
| [`wrong-document`](wrong-document.md) | `usage` |
| [`invalid-feature`](invalid-feature.md) | `usage` |
| [`invalid-text-range`](invalid-text-range.md) | `usage` |

## Text warnings (`lamina-text`)

| Code | Kind |
| --- | --- |
| [`tounicode-conflict`](tounicode-conflict.md) | `usage` |
| [`whole-font-rebuilt`](whole-font-rebuilt.md) | `usage` |
