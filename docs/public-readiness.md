# Hosted runners and the public transition

The owner selected standard GitHub-hosted runners on 2026-11-07. This change replaces
every self-hosted workflow job and adds required native Rust checks on six platforms.
Changing repository visibility remains an owner action, separate from this PR.

## Coverage

| Native Rust platform | Standard runner label |
| --- | --- |
| Linux x64 | `ubuntu-24.04` |
| Linux Arm64 | `ubuntu-24.04-arm` |
| macOS x64 | `macos-15-intel` |
| macOS Arm64 | `macos-15` |
| Windows x64 | `windows-2025` |
| Windows Arm64 | `windows-11-arm` |

These labels come from the [GitHub runner image inventory](https://github.com/actions/runner-images#available-images).
Each matrix leg runs `cargo xtask check rust`: portability, formatting, Clippy, tests,
documentation and dependency checks. `fast-gate` requires the entire matrix.
The full lane also runs Rust and TypeScript checks on Linux Arm64, required by `gate`.
[X6](todos/done/X6-windows-ci.md) retains full TypeScript checks on Windows as follow-up work.

Every other job runs on Ubuntu 24.04 x64. Jobs use `macos-15` Arm64 only when they need a Mac:
WebKit (closest to Safari), the render goldens and the nightly render diff (PDFium draws differently
per platform, and the goldens and the diff's thresholds were measured on macOS), the CLI's macOS
build and its Homebrew check, and the macOS legs of the native Rust matrix. No paid larger
runner labels are used.
Standard runners are free for public repositories; private-repository billing and
runner availability still apply before the visibility change.
See [GitHub's hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Setup on fresh machines

- Rust uses the pinned toolchain, explicit cross targets and architecture-specific caches.
  Only main saves Rust caches. Pull requests can restore main's dependencies;
  [GitHub scopes their caches to their merge refs](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows#restrictions-for-accessing-a-cache), so main cannot restore a fork PR's cache.
- Node and pnpm use the repository pins and lockfile-keyed caches. The old persistent
  pnpm store and Rust-home isolation action are no longer used.
- PDF suites install checksum-verified qpdf 12.4.2 and Python 3.13 explicitly.
  veraPDF installs Java 21. The text differential builds Poppler 26.10.0 from its
  verified source on Linux: the packaged Poppler is not the pinned text oracle.
  Poppler remains a test tool; it is not linked into or shipped with Lamina PDF.
- Fuzz smoke tests run two processes at once to fit the standard runner's memory.
  Benchmark history starts a separate `github-hosted/ubuntu-24.04-x64` baseline.
  Hosted machines vary, so timing regressions remain advisory.
- All workflow scripts use Bash, including the native Windows jobs.
  The runner cleanup workflow is removed because each hosted VM is discarded.

## Audit on 2026-11-07

Gitleaks 8.30.1 scanned all locally available Git refs: 359 commits and approximately
29 MB of history. Four matches were inspected: an encryption test vector and PDF
research formulas/table text. None was a live service credential. This scan covers
Git history, not GitHub issue bodies, comments, logs or uploaded artifacts.

GitHub's push response reported **21 existing Dependabot alerts: 16 high and 5 moderate**.
The alert API returned HTTP 403, so their details could not be read. A root-workspace
`pnpm audit --json` independently found nine advisory IDs: four high and four moderate
in `undici` 7.29.1 through Wrangler/Miniflare, and one moderate in `minimatch` 10.2.x
through TypeDoc/API Extractor. Undici's advisories name 7.30.1 as patched; the minimatch
advisory names 11.0.1. These are existing tooling dependency findings, outside the
runner migration. Triage them and the remaining GitHub alerts before the public release;
the npm audit does not cover Cargo or the separate spike/tool lockfiles.

The root contains both `LICENSE-MIT` and `LICENSE-APACHE`. The fixture inventory
records generated PDF provenance, Noto's OFL and Unicode notices. External corpora
are fetched into ignored folders; their manifest explicitly forbids committing
copies with uncertain redistribution rights. This PR adds the upstream DejaVu
licence notice for the font subset embedded in copies of `text-example.pdf`.

Normal PR workflows use read-only contents permissions and checkouts without stored
credentials. Docs and site deployment previews skip forks. Preview deployment uses
the base branch's configuration, and publishing credentials stay in the release
jobs. The daily release selector validates a completed full main nightly before
checking out its code. No `pull_request_target` workflow executes PR code.

The connected GitHub integration cannot inspect administration settings: workflow
token defaults, outside-contributor approval and secret-scanning alerts returned
HTTP 403. Private vulnerability reporting returned HTTP 404, which does not establish
whether it is configured. Those settings remain owner checks.

## Change visibility in this order

1. Stop and unregister every self-hosted runner before making the repository public.
   Stop the Linux launchd supervisor with `tools/ci-runner/linux/down.sh`; removing
   its registration alone allows the loop to register another runner. Stop the Mac
   runner services too. Cancel queued or running jobs from the old workflows.
2. Review GitHub-only private material that the Git scan cannot cover: issue and PR
   discussions, Actions logs and artifacts, releases, and branches beyond main.
   Review repository and environment secret names and remove unused credentials.
   Do not copy credential values into issues or this document.
   Triage the [existing dependency alerts](https://github.com/laudebugs/laminapdf/security/dependabot).
3. In repository settings, confirm read-only default workflow permissions, approval
   for outside-contributor runs, and required checks `fast-gate` and `gate` with no
   bypass. Confirm deploy credentials have the intended scope and repository access.
4. Change visibility to public. Enable secret scanning, push protection and private
   vulnerability reporting where available; add a security reporting policy with
   the working private reporting route. Check any ruleset or plan changes GitHub
   shows during the visibility change.
5. Run this PR's hosted checks on the current head. Merge only when both gates pass.
   Then run a full nightly and a release dry run, including docs deployment, before
   the next publish. Keep the former runner services stopped and remove their
   obsolete Linux heartbeat variables.

If visibility changes before the migration merges, main still contains the old
self-hosted workflows. Keeping those machines offline closes that transition gap;
the migration branch's required checks can run on the hosted machines.

## Notice source

The DejaVu notice is copied from [DejaVu Fonts 2.37's licence](https://github.com/dejavu-fonts/dejavu-fonts/blob/version_2_37/LICENSE)
into [the fixture notice](../fixtures/fonts/DejaVu-LICENSE.txt). It covers the embedded
font in the fixture, spike and research copies without changing their PDF bytes.
