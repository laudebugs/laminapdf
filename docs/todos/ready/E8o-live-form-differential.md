# E8o — The live form differential: field names, types and values against pypdf and pdf.js

| Field | Value |
| --- | --- |
| ID | `E8o` |
| Status | `ready` |
| Plan | [E8 plan §9](../../plans/editor/e8-form-filling.md#9-tests) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8c](../done/E8c-acroform-field-model.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0018](../../decisions/0018-test-strategy.md) (tier 6, the live differential) |
| Branch | `test/e8o-live-form-differential` |
| PR title | `test(differential): form fields against pypdf and pdf.js over the corpus` |
| Size | M (200–400 of code; the exceptions file beyond) |
| PR | — |

## Goal

The nightly lane checks Lamina PDF's form view against two other readers over the external corpus. `cargo xtask differential --live --forms` compares each terminal field's fully qualified name, type and value with pypdf's and pdf.js's, so a difference in how the form tree is read shows up as a failing night with the file named, as the text, XMP and inspection differentials already do. E8c built the view and its fixture tests; this todo was split from it to keep that PR reviewable.

## Context

- [E8 plan §9](../../plans/editor/e8-form-filling.md#9-tests): the form view's differential row
- [`tools/differential/README.md`](../../../tools/differential/README.md): the live readers, the runner (`tools/xtask/src/runner.rs`), and how the text, XMP and inspection modes compare and excuse differences
- `tools/xtask/src/live/` (`xmp.rs`, `inspect.rs`): the newest modes, a pattern to follow; `tools/xtask/src/changes/differential.rs`, which says when a PR runs a mode sampled
- `crates/laminapdf/examples/differential/inspect-facts.rs`: Lamina PDF's side of a mode, an example that reads a `<name>\t<path>` list
- E8c's `laminapdf::Document::form` and `fixtures/forms/`; what pdf.js reports differently there (it follows a kid's `/Parent` rather than the tree, reads a text-stream value as empty, and gives a button group with `/Opt` its export value)

## Scope

**In scope**

- Lamina PDF's side: a `form-facts` example over `Document::form` (name, kind, value per terminal field, and the reconciliation counts)
- pypdf (`get_fields`) and pdf.js (each page's widget annotations: `fieldName`, `fieldType`, `fieldValue`) as readers in `tools/differential/`
- `--live --forms` in xtask: the comparison (values normalised as each reader writes them), `fixtures/differential/forms-exceptions.toml` with `[[known]]` entries, the report in `target/nightly/differential-forms/`
- The mode in `changes/differential.rs`, the full lane's sampled `live-differentials` step and the nightly's differential job
- A local `--sample` run whose exceptions are committed, each with its reason

**Out of scope** (and which todo covers it)

- Field appearances → [E8d](../done/E8d-field-appearances.md)
- Values after formatting and calculation → [E8f](E8f-declarative-behaviour.md)

## Tasks

- [ ] `crates/laminapdf/examples/differential/form-facts.rs`
- [ ] pypdf and pdf.js readers in `tools/differential/`
- [ ] `tools/xtask/src/live/forms.rs`, its mode and the report
- [ ] `fixtures/differential/forms-exceptions.toml` from a sampled run
- [ ] The workflow steps and `changes/differential.rs`

## Acceptance criteria

- [ ] Field names, types and values match pypdf and pdf.js on the pinned corpus's forms, every difference excused by a `[[known]]` entry with its reason
- [ ] `cargo xtask differential --live --forms --sample 40` passes locally and in the full lane

## Post-merge checks

- [ ] The first nightly run of the mode passes, or its failures become `[[known]]` entries with issues

## Tests

The mode's own unit tests in xtask (normalising each reader's values, matching exceptions), and the sampled run.

## Docs to update

- `tools/differential/README.md`, CLAUDE.md's command list
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos)

## Risks and open questions

- pdf.js reads fields from widgets, so a widgetless field and an orphan widget's parent differ by design; those are excused by class rather than per file

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
