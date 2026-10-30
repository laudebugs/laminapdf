# Hosted CI runners for Linux and Windows: pricing and fit (as of 2026-10-30)

Scope: what it would cost to run lamina's Linux and Windows CI on hosted machines instead of (or next to) the self-hosted Mac runners, comparing **Blacksmith**, **CircleCI** and **GitHub-hosted** runners. Researched on 2026-10-30 for [X6](../../../todos/ready/X6-windows-ci.md) (Windows CI) and the Linux x86_64 gap left by [X5](../../../todos/done/X5-linux-ci-container.md). Prices change: re-check every rate against the sources below, and re-measure the run times (see [How to refresh this note](#how-to-refresh-this-note)), before deciding anything.

---

## 1. Context

- On 2026-10-30, every CI job runs on GitHub Actions on self-hosted runners on one Mac: two macOS ARM64 runners and one Linux ARM64 container (X5). `gate` in `.github/workflows/full.yml` is the one required check.
- Nothing runs on **Linux x86_64** or **Windows**. Work that needs them: X6 (`cargo xtask check` on Windows), D2e (desktop e2e on Linux and Windows), D6b/D6d (release builds, Windows signing), L4d (CLI install checks on Windows, Linux glibc and musl), E10i (Windows certificate store, Linux NSS signers).

## 2. Prices (USD per minute)

| Runner | Blacksmith | CircleCI | GitHub-hosted |
| --- | --- | --- | --- |
| Linux x64, 2 vCPU | $0.004 | $0.006 (Docker medium, 10 credits) | $0.006 |
| Linux x64, 4 vCPU | $0.008 † | $0.012 (Docker large, 20 credits) | $0.012 |
| Linux ARM64, 2 vCPU | $0.0025 | $0.006 (Arm VM medium, 10 credits; Arm Docker medium is 13) | $0.005 |
| Linux ARM64, 4 vCPU | $0.005 † | $0.012 (Arm VM large, 20 credits) | $0.008 |
| Windows x64, 2 vCPU | $0.008 | none (smallest Windows size is 4 CPU) | $0.010 |
| Windows x64, 4 vCPU | $0.016 † | $0.024 (Windows medium, 4 CPU / 16 GB, 40 credits) | $0.022 |
| Windows ARM64 | none listed | none listed | $0.010 (2-core), $0.014 (4-core) |
| Free each month | 3,000 Linux x64 2-vCPU minutes (≈ $12) | 30,000 credits (≈ $18) | Plan's included minutes; larger (4-core+) runners are always billed |
| Plan fee | none | $0 (Free) or from $15/month (Performance) | none |

† Derived, not quoted: Blacksmith's docs say larger runners use the free allowance in proportion to vCPUs (a 4-vCPU minute counts as two 2-vCPU minutes), so this note assumes the price scales the same way. The pricing page lists one rate per OS and doesn't state the scaling.

Notes:
- **CircleCI credits** cost $15 per 25,000 (≈ $0.0006 each), the same rate for prepaid bulk.
- **Blacksmith's advertised savings** ("67%" Linux, "60%" Windows) assume "an estimated 2x faster runtime on Blacksmith". Per minute, at equal size, Blacksmith is about **33% cheaper on Linux x64** and **about 27% cheaper on Windows** (4 vCPU) than GitHub-hosted, not half. The docs' "exactly half the cost of GitHub's per minute" doesn't match GitHub's current rates, which dropped on 2026-01-01.
- **Blacksmith Windows** runners (Windows Server 2025) are labelled **public beta** in the docs; the pricing page shows no beta label. Linux Docker containers don't run on them. No Windows ARM runner. Billing granularity (per second or per minute) isn't stated.
- **GitHub's self-hosted runner charge** ($0.002/min, announced for 2026-03-01) was **postponed** with no new date. The self-hosted Mac runners cost nothing per minute today; re-check this, because it would change the self-hosted vs hosted comparison.

## 3. Measured run times (2026-10-29 and 2026-10-30)

The sample covers the CI, Full lane and Nightly runs created on those two days (291 runs, 5 of whose job lists failed to load). The pace is agent-driven and may not match a normal month.

| Measure | Value |
| --- | --- |
| Code runs per day (CI `Rust` job) | ≈ 39: 25 PR runs + 14 pushes to `main` |
| Code runs per 30 days | ≈ 1,170, of which ≈ 420 pushes to `main` |
| `cargo xtask check` on Linux ARM64 (X5 container, 3 CPUs, rust-cache from `main`), 14 runs | median 5.2 min, p90 7.4 min, max 13.2 min |
| `Rust` job on the macOS runners, 77 runs | median 3.5 min, p90 6.5 min |
| `cargo xtask check` on Windows | **no data** |

Estimates used below: **Linux x64 ≈ 6 min** on 4 vCPU (rounded up from the Linux ARM median). **Windows ≈ 10 min** on 4 vCPU, an assumption (Rust usually builds and links more slowly on Windows); the plausible range is 8–15 min.

## 4. Monthly estimates (30 days, 4-vCPU runners)

| Linux x64 + Windows x64 run on | Blacksmith | CircleCI | GitHub-hosted |
| --- | --- | --- | --- |
| Every code run (≈ 1,170) | **≈ $230** (Linux $56 + Windows $187 − $12 free) | ≈ $360 (Linux $84 + Windows $281 − $18 free + $15 plan) | ≈ $340 (Linux $84 + Windows $257) |
| Linux every code run, Windows on `main` only (≈ 420) | **≈ $110** (Linux $56 + Windows $67 − $12) | ≈ $180 (Linux $84 + Windows $101 − $18 + $15) | ≈ $177 (Linux $84 + Windows $92) |
| Both on `main` only | **≈ $75** (Linux $20 + Windows $67 − $12) | ≈ $130 (Linux $30 + Windows $101 − $18 + $15) | ≈ $123 (Linux $30 + Windows $92) |

- Sensitivity: every extra 5 min of Windows time adds ≈ $94/month (Blacksmith, every code run) or ≈ $34 (`main` only).
- Not included: cancelled runs, first cold builds on each runner, per-minute rounding. Allow 10–20% on top.
- Moving the Linux ARM64 check off the Mac to Blacksmith would add ≈ $35/month at the same volume.
- An earlier estimate in the same session (100 runs × 25 min, ≈ $48 on Blacksmith) assumed too few runs and too long a run; the measured numbers above replace it.

## 5. Fit, beyond price

- **Blacksmith** runners are GitHub Actions runners: the migration is a `runs-on` label (`blacksmith-4vcpu-ubuntu-2404`, `blacksmith-4vcpu-windows-2025`). `gate` stays the only required check, `tauri-action` and GitHub artifact attestations (D6b, L4d) keep working, and the standard `actions/cache` is served from a co-located cache. Whether its cache keeps GitHub's branch scoping (PRs can't write a cache `main` restores) is **not verified**.
- **CircleCI** is a second CI system: a separate config, dashboard and required check, a `changes` job of its own so docs-only PRs still report, and caches that aren't branch-scoped (only `main` should save them). `tauri-action` and GitHub attestations aren't available there, so releases would stay on GitHub Actions. Fork PR builds should be off; secrets go in restricted contexts, with OIDC where the provider supports it.
- **GitHub-hosted** needs no new vendor and offers Windows ARM64, at a price close to CircleCI's.
- All three see the code and any secrets their jobs get.

## 6. Recommendation (opinion, 2026-10-30)

If hosted runners are adopted: **Blacksmith**, with Linux x64 on every code run and Windows on pushes to `main` plus a `ci:windows` label (≈ $110/month), a spending cap set in Blacksmith, and a week of real Windows timings before deciding whether Windows runs on every PR (before Round D's Windows work). Re-check sections 2 and 3 first.

## How to refresh this note

Re-measure run times with `gh` (writes one row per job: date, workflow, event, job, conclusion, seconds, labels):

```sh
gh run list --limit 1000 --created ">=YYYY-MM-DD" --json databaseId,workflowName,event,createdAt \
  --jq '.[] | select(.workflowName=="CI" or .workflowName=="Full lane") | [.databaseId,.workflowName,.event,.createdAt[:10]] | @tsv' |
while IFS=$'\t' read -r id wf ev d; do
  gh api "repos/laudebugs/lamina/actions/runs/$id/jobs?per_page=50" |
    jq -r --arg wf "$wf" --arg ev "$ev" --arg d "$d" '.jobs[] | select(.started_at and .completed_at)
      | [$d,$wf,$ev,.name,.conclusion,((.completed_at|fromdate)-(.started_at|fromdate)),(.labels|join(","))] | @tsv'
done > jobs.tsv
```

## Sources

- Blacksmith pricing: <https://www.blacksmith.sh/pricing> (rates, free minutes, add-ons, the "2x faster runtime" footnote)
- Blacksmith runner docs: <https://docs.blacksmith.sh/blacksmith-runners/overview> (labels, sizes, free-minute conversion, Windows public beta)
- CircleCI pricing: <https://circleci.com/pricing/> and price list: <https://circleci.com/pricing/price-list/>
- GitHub-hosted runner pricing: <https://docs.github.com/en/billing/reference/actions-runner-pricing>
- GitHub's 2026 Actions pricing changes and the self-hosted charge's postponement: <https://github.com/resources/insights/2026-pricing-changes-for-github-actions>
- Run times: GitHub Actions job data for `laudebugs/lamina`, 2026-10-29 to 2026-10-30, collected with the script above.
