# Resolution: CI's package installs failed without retrying

[Diagnosis](detail.md).

## The fix

Each root process now ends itself before the retry's 15-minute timeout, so the retry never has to kill it. `timeout -k 30` sends TERM at its limit and KILL 30 seconds later:

- `linux-webkit`: `dpkg --configure -a` (60 s, to finish an install a cut-off try left half done), `apt-get update` (240 s) and `apt-get install` (420 s), each under `sudo timeout`; 13.5 minutes at most;
- `playwright-deps`: Playwright runs as root under `sudo env "PATH=$PATH" timeout -k 30 780`, so it calls apt directly, not through its own `sudo`.

Both also wait 60 seconds between tries.

A try also ends early when the mirror is out ([#429](https://github.com/laudebugs/laminapdf/pull/429)). `.github/actions/apt-fail-fast` gives each apt download 30 s and 3 tries. Its `update.sh` runs `apt-get update` and fails the try at once when an Ubuntu index can't be fetched; apt itself only warns. A third-party source the runner image lists, such as Microsoft's packages, still only warns, so one of them being down doesn't fail every Linux job. On 2026-11-11's failing logs the check matches four lines in each job; on a healthy run it matches none.

This turns the crash into ordinary retries; it doesn't outlast a long outage. The one on 2026-11-11 lasted about an hour, longer than three tries, and the jobs with a 30-minute limit (`pdf-runtimes`, `msrv-and-spike`) end before a third try. Such a run still fails, and a rerun once the mirrors answer passes.

## How it was checked

`actionlint` passes, and `cargo xtask doctor`'s test still finds the action's package list. Only an outage on GitHub's runners can show the retry working; until one comes, the check is that the jobs still pass.

## Left for later

- `playwright-chromium` installs Chromium's packages with `--with-deps`, through `sudo` under the same retry, so it could hit the same `kill EPERM`. On 2026-11-11 its tries failed quickly instead (apt exited with code 100) and ran out with the outage.
- The release workflow's inline installs and the `poppler` action run `sudo apt-get` with no retry: no `kill EPERM`, but no retry either.
