#!/usr/bin/env bash
# One try of the apt-install action: run inside its retry, so each root process ends itself under
# `sudo timeout` (KILL 30 s after TERM) before the retry's 30-minute (1,800 s) limit, which must
# never have to kill a root process (docs/bugs/ci-apt-retry-kill-eperm). At most, a try takes
# 270 s (dpkg) + 270 s (update.sh) + $APT_TIMEOUT + 30 s (install) + 90 s (autoclean):
# 1,560 s at the default 900 s, 1,760 s at the 1,100 s cap. The .deb files go to (and come
# from) $APT_ARCHIVES, the folder the action caches.
set -euo pipefail
if ! [[ "$APT_TIMEOUT" =~ ^[0-9]+$ ]] || [ "$APT_TIMEOUT" -gt 1100 ]; then
  echo "::error::apt-install's timeout is $APT_TIMEOUT s; it takes whole seconds up to 1,100"
  exit 1
fi
# A try cut off mid-install leaves dpkg half done, and every apt call after it fails with "dpkg
# was interrupted" (docs/bugs/ci-apt-slow-or-unreachable-mirror): finish that install first.
sudo timeout -k 30 240 dpkg --configure -a
bash "$APT_UPDATE"
# The runner owns the folder, so apt downloads as root rather than as `_apt` (a notice, not a
# failure); the cache restores the files as the runner. `$PACKAGES` unquoted: one argument each.
# shellcheck disable=SC2086
sudo timeout -k 30 "$APT_TIMEOUT" apt-get install -y -q --no-install-recommends \
  -o Dir::Cache::Archives="$APT_ARCHIVES" -o APT::Keep-Downloaded-Packages=true \
  -o DPkg::Lock::Timeout=60 $PACKAGES
# Drop the files no index offers any more (an older image's versions), so the saved cache holds
# only what the next run on this image can use. The packages are installed: a failure here only
# leaves a larger cache.
sudo timeout -k 30 60 apt-get autoclean -q -o Dir::Cache::Archives="$APT_ARCHIVES" \
  || echo "::warning::apt-get autoclean failed; the cache keeps every downloaded file"
