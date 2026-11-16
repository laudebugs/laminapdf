# The daily release fails with a 500 after its tree is built

[Resolution](resolution.md).

## What we saw

On 2026-11-16 the Daily release run [43895953056](https://github.com/laudebugs/laminapdf/actions/runs/43895953056/job/133399447103) failed in its `prepare` job, at the step "Publish the signed version commit and tag", for `v0.5.0`:

```text
gh: HTTP 500
jq: parse error: Invalid numeric literal at line 1, column 10
```

The log has no "Tree request failed" warning, so the tree requests had passed: GitHub failed a later request. The step creates no refs until the commit is verified, and neither `release/v0.5.0` nor `v0.5.0` exists, so it left nothing behind.

## How to reproduce

A GitHub 500 isn't reproducible on demand. A stand-in `gh` that answers 500 to a request shows the step stopping at that request.

## Cause

Only the tree requests were retried ([`release-tree-request-502`](../release-tree-request-502/resolution.md) left the rest for later). The binary blob upload, the commit request, the commit's read-back and the two ref requests each ran once, so one 500 from GitHub on any of them stopped the release. The 500's body, not JSON, then failed `jq`.
