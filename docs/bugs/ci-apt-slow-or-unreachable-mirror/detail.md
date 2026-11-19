# CI's package installs failed on a slow mirror and on dropped downloads

[Resolution](resolution.md). It follows [`ci-apt-retry-kill-eperm`](../ci-apt-retry-kill-eperm/detail.md), whose fix made each apt call end itself under `sudo timeout` so the retry could try again.

## What we saw

On 2026-11-18 and 19, two runs failed in their apt installs, each in a different way:

1. **Release run 44366942813.** `playwright install-deps` downloaded 32 MB from `us-east-2.ec2.archive.ubuntu.com` at 60 kB/s. At 540 s its `sudo timeout` ended it while dpkg was configuring the packages. Both retries then failed at once with `E: dpkg was interrupted, you must manually run 'sudo dpkg --configure -a'`.
2. **Nightly run 44521657058, job 135551537387 ("Signed saves").** The step ran a bare `sudo apt-get update && sudo apt-get install poppler-utils`, with no retry. The indexes came down, but both .deb downloads failed with "Connection failed" on every mirror the image lists: azure, archive and security.

## How to reproduce

It needs a slow or failing Ubuntu mirror on GitHub's runners, which we can't cause. Both runs' logs show the pattern.

## Cause

Three gaps lay behind the two failures:

- **A cut-off try broke the next one.** `playwright-deps` didn't repair dpkg at the start of a try, as `linux-webkit` and `desktop-build` did. A try that ended mid-configure left dpkg interrupted, and apt refuses every install until `dpkg --configure -a` runs.
- **Every run downloaded everything.** Each job fetched its .deb files from the mirrors again, so any slow or failing mirror reached every job. At 60 kB/s, 32 MB takes about 9 minutes, longer than the 540 s each try had.
- **Some installs had no retry, and the mirror list had no fallback outside Ubuntu's own hosts.** The nightly's `pdfsig` install, the release lane's two WebKitGTK installs, `poppler` and `differential-tools` ran `sudo apt-get` once. The image's `/etc/apt/apt-mirrors.txt` lists only Ubuntu's mirrors, and on 2026-11-19 all of them refused the same downloads.
