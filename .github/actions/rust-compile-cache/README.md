# Rust compile cache

This action lets a CI job reuse Rust crates that an earlier run compiled, so a pull request recompiles only the crates its change reaches. It runs [sccache](https://github.com/mozilla/sccache) (Apache-2.0, pinned in `tools/xtask/src/doctor.rs`) under every `rustc` call and keeps the compiled crates in a bucket on Cloudflare R2. Pushes to `main` fill the bucket; every other run only reads it. The [CI caching plan](../../../docs/plans/ci-caching.md) explains the choices, and [ADR 0018 §2](../../../docs/decisions/0018-test-strategy.md#2-ci-lanes) lists the workflows that may never use it.

## What it does

A job calls the action twice:

```yaml
    # On a push to main, the job takes the environment that holds the token that writes.
    environment: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' && 'rust-compile-cache' || '' }}
    steps:
      # … checkout, then the toolchain step (actions-rust-lang/setup-rust-toolchain or ./.github/actions/host-rust)
      - uses: ./.github/actions/rust-compile-cache
        with:
          endpoint: ${{ vars.SCCACHE_R2_ENDPOINT }}
          bucket: ${{ vars.SCCACHE_R2_BUCKET }}
          access-key-id: ${{ secrets.SCCACHE_R2_ACCESS_KEY_ID }}
          secret-access-key: ${{ secrets.SCCACHE_R2_SECRET_ACCESS_KEY }}
      # … the build
      - if: ${{ !cancelled() }}
        uses: ./.github/actions/rust-compile-cache
        with:
          stats: "true"
```

The first call comes after the toolchain step, because the Rust cache puts every `RUST*` variable into its key. A `RUSTC_WRAPPER` set earlier would give runs with the compile cache and runs without it (fork pull requests) different keys. A test in `tools/xtask/src/workflows/compile_cache.rs` checks the order.

The first call:

1. installs sccache through `.github/actions/install-tools`;
2. starts sccache's server with the R2 credentials, which only that step's environment holds. The server starts each `rustc` with the job's environment rather than its own, so later steps, build scripts and proc-macros don't get the credentials in their environment. A process of the same user could still read the server's environment from the operating system; on a pull request that would give it only the read-only token;
3. sets the mode: `READ_WRITE` on a push to `main`, `READ_ONLY` everywhere else;
4. exports `RUSTC_WRAPPER=sccache` and `SCCACHE_IGNORE_SERVER_IO_ERROR=1`. If the server fails partway through, the build goes on: sccache starts a new server without the credentials, which caches only on the runner's disk, or runs `rustc` directly.

Without the variables or secrets, the first call prints a notice and exports nothing, and the job builds as it would without the action. The second call writes `sccache --show-stats` into the job summary: hits, misses, and why some calls weren't cacheable.

The compiled crates are stored under the key prefix `rust/v1/`. To start the compile cache afresh, change it to `rust/v2/` in `action.yml`.

## The owner's setup

### Cloudflare

1. Create an R2 bucket for the compile cache alone.
2. Add a lifecycle rule to the bucket that deletes objects 30 days after upload. Entries still in use are uploaded again by the next push to `main`.
3. Create two R2 API tokens, each limited to that bucket: one with **Object Read only**, one with **Object Read & Write**. Each gives an access key ID and a secret access key.

### GitHub

| Name | Kind | Where | Value |
| --- | --- | --- | --- |
| `SCCACHE_R2_ENDPOINT` | variable | repository | `https://<account id>.r2.cloudflarestorage.com` |
| `SCCACHE_R2_BUCKET` | variable | repository | the bucket's name |
| `SCCACHE_R2_ACCESS_KEY_ID` | secret | repository, and Dependabot | the read-only token's access key ID |
| `SCCACHE_R2_SECRET_ACCESS_KEY` | secret | repository, and Dependabot | the read-only token's secret access key |
| `SCCACHE_R2_ACCESS_KEY_ID` | secret | the `rust-compile-cache` environment | the read-and-write token's access key ID |
| `SCCACHE_R2_SECRET_ACCESS_KEY` | secret | the `rust-compile-cache` environment | the read-and-write token's secret access key |

Create the `rust-compile-cache` environment with a deployment branch rule that allows only `main`. An environment's secrets take precedence over the repository's, so a job reads the same two names everywhere and gets the token that writes only inside that environment. A pull request runs on `refs/pull/<n>/merge`, which the branch rule refuses, so a pull request can't take the environment even when it edits a workflow.

Fork pull requests get no secrets, so they compile without the compile cache.
