# Owner-controlled GitHub Actions

Repository policy permits only actions owned by `laudebugs`, pinned to a full commit SHA.
The workflows therefore use forks of the upstream actions below. Inline comments record
the reviewed upstream release for each SHA.

| Fork | Upstream |
| --- | --- |
| `laudebugs/action-setup` | [`pnpm/action-setup`](https://github.com/pnpm/action-setup) |
| `laudebugs/cache` | [`actions/cache`](https://github.com/actions/cache) |
| `laudebugs/checkout` | [`actions/checkout`](https://github.com/actions/checkout) |
| `laudebugs/crates-io-auth-action` | [`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action) |
| `laudebugs/create-github-app-token` | [`actions/create-github-app-token`](https://github.com/actions/create-github-app-token) |
| `laudebugs/download-artifact` | [`actions/download-artifact`](https://github.com/actions/download-artifact) |
| `laudebugs/install-action` | [`taiki-e/install-action`](https://github.com/taiki-e/install-action) |
| `laudebugs/setup-java` | [`actions/setup-java`](https://github.com/actions/setup-java) |
| `laudebugs/setup-node` | [`actions/setup-node`](https://github.com/actions/setup-node) |
| `laudebugs/setup-python` | [`actions/setup-python`](https://github.com/actions/setup-python) |
| `laudebugs/setup-rust-toolchain` | [`actions-rust-lang/setup-rust-toolchain`](https://github.com/actions-rust-lang/setup-rust-toolchain) |
| `laudebugs/upload-artifact` | [`actions/upload-artifact`](https://github.com/actions/upload-artifact) |

Create the forks once with an owner-authorized GitHub CLI login:

```sh
test "$(gh api user --jq .login)" = laudebugs || {
  echo "authenticate gh as laudebugs before creating the action forks" >&2
  exit 1
}

for upstream in \
  actions/checkout \
  actions/cache \
  actions/create-github-app-token \
  actions/download-artifact \
  actions/setup-java \
  actions/setup-node \
  actions/setup-python \
  actions/upload-artifact \
  actions-rust-lang/setup-rust-toolchain \
  pnpm/action-setup \
  rust-lang/crates-io-auth-action \
  taiki-e/install-action
do
  fork="laudebugs/${upstream#*/}"
  gh repo view "$fork" >/dev/null 2>&1 || gh repo fork "$upstream" --clone=false
done
```

Before merging an action update, sync its fork from the named upstream repository and
review the upstream release. Keep the workflow reference at the reviewed 40-character
commit SHA. The `external_actions_should_be_owner_controlled_and_commit_pinned` test
checks every workflow and composite action, including the reviewed release copies.

After creating or syncing forks, verify every referenced commit is available:

```sh
rg --no-filename -o 'laudebugs/[^@ ]+@[0-9a-f]{40}' .github tools/release --glob '*.yml' |
  sort -u |
  while IFS=@ read -r action sha; do
    repository=${action#laudebugs/}
    repository=${repository%%/*}
    gh api "repos/laudebugs/$repository/git/commits/$sha" --silent
  done
```
