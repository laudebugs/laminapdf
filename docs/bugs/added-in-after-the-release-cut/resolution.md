# Resolution

The other 216 Rust items, the 56 TypeScript symbols and the 21 codes now name 0.3.0, the next release: the pending changesets are minor, and 0.x follows SemVer since the alphas ended (ADR 0017). The items were found by keying each public item of rustdoc's JSON by its path at the tag and on `main`, and each TypeScript `@since` by whether the symbol existed at the tag.

`cargo xtask check since` and `gen codes --check` pass on the result; neither can tell 0.2.0 from 0.3.0, so the list above is what proves it.

## Left for later

A check that each public item new since the newest `v*` tag names a version after that tag would catch this when a PR merges after a release cut. It needs rustdoc's JSON at the tag as well as at the PR, so it belongs in the full lane, not in `check since`.
