# The daily release fails with a 502 while publishing its signed commit

[Resolution](resolution.md).

## What we saw

On 2026-11-10 the Daily release run [42858624993](https://github.com/laudebugs/laminapdf/actions/runs/42858624993/job/129978388361) failed in its `prepare` job, at the step "Publish the signed version commit and tag", for `v0.2.0-alpha.5`. The step ran for three minutes, then GitHub answered one request with `Server Error (HTTP 502)`. The script read a body with no `sha`, and `jq` stopped it:

```text
gh: Server Error (HTTP 502)
jq: error (at <stdin>:3): null (null) cannot be matched, as it is not a string
```

The step creates no refs until the commit is verified, so the failure left no release branch or tag behind.

## Measurements

The step sent every changed file in one request to GitHub's trees API, with each text file's content inline. That request grows with every release, because the release commit carries the generated package docs.

| Release | Files changed | Content sent | Step time |
| --- | --- | --- | --- |
| `v0.2.0-alpha.4` (2026-11-09) | 289 | 1.0 MiB | 2 min 11 s, passed |
| `v0.2.0-alpha.5` (2026-11-10) | not measured | not measured | 3 min 7 s, then 502 |

The step's log has no timestamps between its requests, so the table can't split the time. Building the entries is local work that takes seconds, which leaves the trees request as the likely cost: it carries all the content.

## How to reproduce

The 502 isn't reproducible on demand: it depends on GitHub's load and on the release's size. The failing run's evidence is its log (above). The step's own logic can be run against any release commit with a stand-in `gh`, as [the resolution](resolution.md#how-it-was-checked) describes.

## Cause

One trees request carried the whole release commit. GitHub took up to two minutes over yesterday's tree, and today's larger one ran past GitHub's limit, so its gateway answered 502. Nothing retried the request, though building a tree is safe to repeat: the same entries on the same base always give the same SHA.
