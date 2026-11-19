# CI's package installs failed while only Ubuntu's security mirror was down

[Resolution](resolution.md).

## What we saw

On 2026-11-19, from about 13:10 to 13:20 UTC, two pull-request jobs failed in their package installs: MSRV on #755 and the sampled live differentials on #756. Each log shows `Err: http://security.ubuntu.com/ubuntu noble-security InRelease  Connection failed [IP: 104.20.28.246 80]`. In the same `apt-get update`, every index from the archive mirror (`us-east-2.ec2.archive.ubuntu.com`: `noble`, `noble-updates`, `noble-backports`) was fetched. Then `apt-fail-fast` warned "apt couldn't fetch Ubuntu's package indexes; this try ends here". All three tries ended the same way over about five minutes, so the step failed. The differentials then failed for want of `exiftool` and `pdffonts`, and MSRV never ran.

## How to reproduce

It needs `security.ubuntu.com` to be unreachable from a runner while the archive answers, which we can't cause. The two logs are jobs `135583143899` (run `44530634235`) and `135584064695` (run `44530890314`). Running `update.sh`'s check on the first try's output of either, in place of `apt-get update`, ends the try with exit code 1.

## Measurements

- Of the 112 failed jobs on 2026-11-19 (gates left out), these 2 failed this way. The others failed for other reasons, including two different mirror problems that this bug doesn't cover. One was the archive mirror itself answering `503 Service Unavailable` for every index, where failing fast is right. The other was downloads so slow that the install hit its timeout (exit code 124), which [`ci-apt-slow-or-unreachable-mirror`](../ci-apt-slow-or-unreachable-mirror/detail.md) is about.
- The packages these jobs install come from `noble` and `noble-updates`, both fetched.

## Cause

`update.sh` ends a try when any `Failed to fetch` line names an Ubuntu host (`[^ ]*ubuntu\.com`) or the image's mirror list. `security.ubuntu.com` is an Ubuntu host, so its outage ended every try, though the install didn't need it. Ubuntu copies packages from the `-security` pocket to `-updates` frequently, as the [Ubuntu Security Team FAQ](https://wiki.ubuntu.com/SecurityTeam/FAQ) says, and the archive mirror serves `-updates`. Each version these jobs install was in `noble` or `noble-updates`.
