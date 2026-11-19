# Resolution: CI's package installs failed while only Ubuntu's security mirror was down

[Diagnosis](detail.md).

## The fix

`apt-fail-fast/update.sh` still ends a try when an Ubuntu index can't be fetched, unless every failed index belongs to a `-security` suite. Then it warns ("apt couldn't fetch Ubuntu's security index; -updates carries the same fixes, so the install goes on") and lets the install run. The rule names the suite, not the host, so it holds on Arm64 too, where `ports.ubuntu.com` serves the security pocket beside the others. An archive index failing ends the try as before, even when the security index fails too.

## How it was checked

`update.sh`'s check ran on six `apt-get update` outputs, with `apt-get update` replaced by the saved output:

| Output | Before | After |
| --- | --- | --- |
| Job `135583143899`'s first try (security down, archive up) | ends the try | warns, goes on |
| Job `135584064695`'s first try (the same) | ends the try | warns, goes on |
| A clean update | goes on | goes on |
| The mirror list's `noble` index failing | ends the try | ends the try |
| `ports.ubuntu.com`'s `noble-security` failing | ends the try | warns, goes on |
| `ports.ubuntu.com`'s `noble-updates` and `security.ubuntu.com` failing | ends the try | ends the try |

`shellcheck` passes.

## Left for later

- An install that needs a package only `-security` holds, newer than its `-updates` copy for a moment, would still fail in `apt-get install`. That gap is minutes long, and the retry tries again a minute later.
- The archive mirror's own outages (the 503s and slow downloads of the same day) still fail jobs once three tries run out, as [`ci-apt-slow-or-unreachable-mirror`](../ci-apt-slow-or-unreachable-mirror/resolution.md) describes.
