# Resolution: CI's package installs failed on a slow mirror and on dropped downloads

[Diagnosis](detail.md).

## The fix

Every Linux apt install in the workflows and setup actions now goes through one composite action, `.github/actions/apt-install`. It takes the package list and a per-try timeout (900 s by default, at most 1,100 s). It works in four steps:

1. **It makes apt fail fast.** `apt-fail-fast` gives each download 30 s and 3 tries. Its `config.sh` now also adds Kernel.org's CDN-backed mirror (`http://mirrors.edge.kernel.org/ubuntu/`) to `/etc/apt/apt-mirrors.txt`, after the image's own entries and with a lower priority than any of them, so apt tries it only when they fail. It does this only on x64 runners, since the mirror doesn't carry the Arm64 ports archive, and only when the image's entries carry priorities: without them, a new entry could rank level with theirs.
2. **It restores the cached .deb files.** `actions/cache` holds the folder apt downloads into (`-o Dir::Cache::Archives`, under the runner's home). The key is `apt-<ImageOS>-<arch>-<hash of the sorted package list>-<ImageVersion>`, and an older image's cache for the same list restores too: apt reuses its files where their versions are still current. Only pushes and scheduled runs on `main` save, as `node-pnpm` does, right after the install and before the job's own steps. Before the save, the action removes the root-only `lock` and the `_apt`-owned `partial/` apt leaves in the folder and gives the folder back to the runner, which the cache tars as; anything still unreadable gets a warning that names the cache.
3. **It installs, in up to three tries a minute apart** under `nick-fields/retry`. Each try runs `dpkg --configure -a` first, so a try after a timeout finishes the cut-off install instead of failing at once. Then it runs `apt-fail-fast/update.sh` and `apt-get install`, each under `sudo timeout -k 30`. The retries run `update.sh` by the path `apt-fail-fast` outputs, the action's own copy: a release dispatch can check out another ref, whose copy may differ or be missing. Each `sudo timeout` adds 30 s of grace before KILL, so a try takes at most 1,560 s at the default 900 s: 270 s for dpkg, 270 s for `update.sh`, 930 s for the install and 90 s for the autoclean. At the 1,100 s cap the sum is 1,760 s, inside the retry's 1,800 s per attempt, so the retry never has to kill a root process.
4. **It trims the cache.** `apt-get autoclean` drops the files no index offers any more, so a saved cache holds only what the image can use.

`playwright-deps` still lets Playwright choose the packages. `playwright install-deps --dry-run <browsers>` simulates the install with `apt-get install -s` and prints the packages it would install, dependencies included, under `Missing system dependencies (N):`. The action reads that list, checks that it found N names, and installs them through `apt-install`. The dry run needs fresh indexes, so it runs after `update.sh` in a retry of its own.

The other installs moved without changing their packages: `linux-webkit`, `desktop-build`'s packaging tools, `poppler`'s build libraries, `differential-tools`' ExifTool (still pinned at `12.76+dfsg-1`) and the nightly's `poppler-utils`. The release lane's two inline WebKitGTK installs now use `linux-webkit`, so the list has one home.

## How it was checked

`actionlint` and `shellcheck` pass, and `gh actions-lock --verify` finds the lockfile valid. The dry-run parsing was run against the output format in Playwright 1.64.1's source (missing packages, none missing, and a failed simulation). `cargo test -p xtask` checks that `linux-webkit` installs doctor's desktop list and that the release lane installs it through `linux-webkit`. The cache saves only once a push to `main` runs the action; it is confirmed when that run shows an `apt-…` cache entry. A second run on the same image that downloads nothing is still to be seen on `main`. Only a slow or failing mirror on GitHub's runners can show the retries and the fallback mirror working; until one comes, the check is that the jobs still pass.

## Left for later

- `playwright-chromium` still installs Chromium's packages with `--with-deps`, through Playwright's own `sudo`, as [`ci-apt-retry-kill-eperm`](../ci-apt-retry-kill-eperm/resolution.md) noted.
- An outage longer than three tries still fails the run, but a rerun passes once the mirrors answer.
- Arm64 runners have no mirror outside Ubuntu's: the Kernel.org mirror doesn't carry the ports archive.
- Three worst-case tries take about 93 minutes, longer than any job's limit. The tightest jobs, at 30 minutes (`ci.yml`'s `msrv-and-spike` and `pdf-runtimes`, the nightly's `signed-saves`, the release lane's `pdf-runtimes`), fit one slow try and part of a second; GitHub then cancels the job. A healthy install takes about a minute.
