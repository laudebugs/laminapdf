# Resolution: CI's package installs failed without retrying

[Diagnosis](detail.md).

## The fix

Each root process now ends itself before the retry's 15-minute timeout, so the retry never has to kill it:

- `linux-webkit`: `sudo timeout 240 apt-get update` and `sudo timeout 600 apt-get install`;
- `playwright-deps`: Playwright runs as root under `sudo env "PATH=$PATH" timeout 780`, so it calls apt directly, not through its own `sudo`.

Both also wait 60 seconds between tries, so a short outage can pass.

## How it was checked

`actionlint` passes, and `cargo xtask doctor`'s test still finds the action's package list. Only an outage on GitHub's runners can show the retry working; until one comes, the check is that the jobs still pass.

## Left for later

`playwright-chromium` (`--with-deps`) and the release workflow's inline installs keep their old form. Today's failures there were the outage itself: their tries failed quickly and ran out, with no kill.
