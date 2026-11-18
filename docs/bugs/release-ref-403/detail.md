# The release trains fail with a 403 creating their refs

[Resolution](resolution.md).

## What we saw

On 2026-11-18 two release workflows failed within minutes of each other, after the same nightly:

- The Daily release run [44220922759](https://github.com/laudebugs/laminapdf/actions/runs/44220922759) for `v0.6.0`, at "Publish the signed version commit and tag". The blob, tree and commit requests passed; creating `refs/heads/release/v0.6.0` answered `gh: Resource not accessible by integration (HTTP 403)`, and the check that the ref exists anyway answered 404.
- The Web reader release run [44220922788](https://github.com/laudebugs/laminapdf/actions/runs/44220922788) for `web-v0.1.9` on 4543a46, at "Tag the release": the same 403.

Neither left a ref behind. Both trains last passed on 2026-11-16.

## Cause (likely, not proven)

Both refs point at the commit the nightly tested, 4543a46. Since it, `main` has changed `docs.yml`, `full.yml`, `nightly.yml` and `site.yml` (#721, #727). GitHub refuses an App token without the `workflows` permission a ref that brings in workflow changes, with this 403, and every train's token asked for `contents: write` alone. The token could write git objects, so it wasn't missing `contents`.

One fact doesn't fit: on 2026-11-16 the web train made `web-v0.1.8` on fd425513, while #681 and #684 had changed workflows on `main` after it. Either GitHub's check is narrower than described here, or a setting outside the repository changed between the two days (the App's permissions, or a ruleset on tags and `release/*` branches). Neither can be seen from the repository.

## How to reproduce

Not on demand: it needs the App's token and a train run whose nightly commit is behind a workflow change on `main`.
