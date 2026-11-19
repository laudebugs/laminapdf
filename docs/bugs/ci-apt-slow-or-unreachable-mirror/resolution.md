# Resolution: CI's package installs failed on a slow mirror and on dropped downloads

[Diagnosis](detail.md).

## The fix

Every Linux apt install in the workflows and setup actions now goes through one composite action, `.github/actions/apt-install`. It takes the package list and a per-try timeout (900 s by default, at most 1200). It works in four steps:

1. **It makes apt fail fast.** `apt-fail-fast` gives each download 30 s and 3 tries. Its `config.sh` now also adds Kernel.org's CDN-backed mirror (`http://mirrors.edge.kernel.org/ubuntu/`) to `/etc/apt/apt-mirrors.txt`, after the image's own entries and with a lower priority than any of them, so apt tries it only when they fail. It does this on x64 runners only: the mirror doesn't carry the Arm64 ports archive.
2. **It restores the cached .deb files.** `actions/cache` holds the folder apt downloads into (`-o Dir::Cache::Archives`, under the runner's home). The key is `apt-<ImageOS>-<arch>-<hash of the sorted package list>-<ImageVersion>`, and an older image's cache for the same list restores too: apt reuses its files where their versions are still current. Only pushes and scheduled runs on `main` save, as `node-pnpm` does, right after the install and before the job's own steps.
3. **It installs, in up to three tries a minute apart** under `nick-fields/retry`. Each try runs `dpkg --configure -a` first, so a try after a timeout finishes the cut-off install instead of failing at once. Then it runs `apt-fail-fast/update.sh` and `apt-get install`, each under `sudo timeout -k 30`. A try takes 1,470 s at most with the default timeout, inside the retry's 30-minute limit.
4. **It trims the cache.** `apt-get autoclean` drops the files no index offers any more, so a saved cache holds only what the image can use.

`playwright-deps` still lets Playwright choose the packages. `playwright install-deps --dry-run <browsers>` simulates the install with `apt-get install -s` and prints the packages it would install, dependencies included, under `Missing system dependencies (N):`. The action reads that list, checks that it found N names, and installs them through `apt-install`. The dry run needs fresh indexes, so it runs after `update.sh` in a retry of its own.

The other installs moved without changing their packages: `linux-webkit`, `desktop-build`'s packaging tools, `poppler`'s build libraries, `differential-tools`' ExifTool (still pinned at `12.76+dfsg-1`) and the nightly's `poppler-utils`. The release lane's two inline WebKitGTK installs now use `linux-webkit`, so the list has one home. `apt-fail-fast` outputs the path of its `update.sh`, so the retries run the script from the action's own copy, not from the checkout.

## How it was checked

`actionlint` and `shellcheck` pass, and `gh actions-lock --verify` finds the lockfile valid. The dry-run parsing was run against the output format in Playwright 1.64.1's source (missing packages, none missing, and a failed simulation). `cargo test -p xtask` checks that `linux-webkit` installs doctor's desktop list and that the release lane installs it through `linux-webkit`. Only a slow or failing mirror on GitHub's runners can show the retries and the fallback mirror working; until one comes, the check is that the jobs still pass and that a second run on the same image downloads nothing.

## Left for later

- `playwright-chromium` still installs Chromium's packages with `--with-deps`, through Playwright's own `sudo`, as [`ci-apt-retry-kill-eperm`](../ci-apt-retry-kill-eperm/resolution.md) noted.
- An outage longer than three tries still fails the run, and a rerun once the mirrors answer passes.
