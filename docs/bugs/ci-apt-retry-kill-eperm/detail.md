# CI's package installs failed without retrying

[Resolution](resolution.md).

## What we saw

On 2026-11-11, main's Linux jobs failed in two setup actions, `linux-webkit` and `playwright-deps`: Rust on x64 and ARM64, MSRV, the SDK's Chromium and Firefox tests, the full lane's ARM64 check and the desktop build. 10 of the day's 12 failed jobs on main failed this way. Each log shows `Temporary failure resolving 'azure.archive.ubuntu.com'` (or `ports.ubuntu.com`), then nothing for about 12 minutes, then `Error: kill EPERM`. The step failed after its first try, though both actions ask for three.

## How to reproduce

It needs the runners' package mirrors to be unreachable, which we can't cause. The pattern is visible in any job of runs `43042915171` and `43038652755`.

## Cause

GitHub's runners couldn't resolve Ubuntu's mirrors for about half an hour. With no mirror, `apt-get install` spent its time on each package's download, past the retry action's 15-minute timeout. The retry action (`nick-fields/retry`) then tried to kill the command, but apt ran as root through `sudo`, and a normal user can't signal a root process. The kill failed with `EPERM`, the action crashed, and the step failed without its second and third tries.
