# The daily release fails with a 500 after its tree is built

[Resolution](resolution.md).

## What we saw

On 2026-11-16 the Daily release run [43895953056](https://github.com/laudebugs/laminapdf/actions/runs/43895953056/job/133399447103) failed in its `prepare` job, at the step "Publish the signed version commit and tag", for `v0.5.0`:

```text
gh: HTTP 500
jq: parse error: Invalid numeric literal at line 1, column 10
```

The log has no "Tree request failed" warning, so the tree requests had passed: GitHub failed a later request. The step creates no refs until the commit is verified, and neither `release/v0.5.0` nor `v0.5.0` exists, so it left nothing behind.

## What the log rules out

The `jq` parse error came right after the 500, so the request's body went straight into `jq`. In the step as it was, only two requests piped into `jq`: the commit request and a binary file's blob upload. The commit's read-back was assigned first, so a failure there stopped the step before `jq` ran, and the two ref requests went to `/dev/null`. The release commit is mostly text (the package docs and version files), so the commit request is the likelier one.

## How to reproduce

A GitHub 500 isn't reproducible on demand. A stand-in `gh` that answers 500 to a request shows the step stopping at that request.

## Cause

Only the tree requests were retried ([`release-tree-request-502`](../release-tree-request-502/resolution.md) left the rest for later). The binary blob upload, the commit request, the commit's read-back and the two ref requests each ran once, so one 500 from GitHub on any of them stopped the release. Here it was the commit or a blob request, whose 500 body, not JSON, then failed `jq`.
