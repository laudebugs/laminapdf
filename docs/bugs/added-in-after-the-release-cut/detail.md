# Items merged after 0.2.0 say they were added in 0.2.0

## What we saw

0.2.0 was tagged at `c892e2db`, and seven feature PRs merged after it: annotation views and ops (#526), page labels (#533), the page-structure ops (#534), general XMP editing (#518), duplicating pages (#543), outline editing (#535) and the initial view (#516). Their public items still said `Added in 0.2.0.` in Rust and `@since 0.2.0` in `@laminapdf/sdk`, and their error codes had `"since": "0.2.0"` in `fixtures/codes.json`. None of them is in the published 0.2.0: someone reading the docs of 0.3.0 would look for them in 0.2.0 and not find them.

## How many

Comparing rustdoc's JSON of every published crate at the tag with `main` (each public item keyed by its path), 229 items carry the marker on `main` and have no counterpart at the tag:

| Crate | Items |
| --- | ---: |
| `laminapdf-model` | 106 |
| `laminapdf-ops` | 94 |
| `laminapdf` | 29 |

Thirteen of the model's are `PageSize`, its constants and its methods, which moved from `laminapdf-build` to `laminapdf-model` in #534; they were in 0.2.0 under the builder, so they keep their marker. In `@laminapdf/sdk`, 56 exports and methods are new (the page, XMP, outline, label and initial-view edits and their types), and 21 codes are new in `codes.json`.

## Cause

Each PR was written before the release was cut, when 0.2.0 was the next release, and named it as `docs/guidelines/rust.md` asks. The release lane tagged an earlier commit, the one the last full nightly passed, so the PRs that merged in between went out after it. Nothing checks a marker against the releases already tagged: `cargo xtask check since` asks only that every public item has a marker.
