#!/usr/bin/env bash
# `apt-get update` that fails when an Ubuntu index can't be fetched (docs/bugs/
# ci-apt-retry-kill-eperm). By default apt only warns, and the install after it spends the rest
# of the try on a mirror it can't reach. Third-party sources the runner image lists (Microsoft's
# packages and others) still only warn: failing on them would fail every Linux job while one is
# down. Run inside the setup action's retry, so a failed try is tried again a minute later.
set -euo pipefail
bash "$(dirname "$0")/config.sh"
log=$(mktemp)
trap 'rm -f "$log"' EXIT
sudo timeout -k 30 240 apt-get update -q 2>&1 | tee "$log"
# The failing source, not apt's reason, decides: noble names Ubuntu's mirrors through
# mirror+file:/etc/apt/apt-mirrors.txt, and a timeout's reason may name only an IP address.
if grep -qE '^(W|E): Failed to fetch (mirror\+file:/etc/apt/apt-mirrors\.txt|[^ ]*ubuntu\.com)' "$log"; then
  echo "::warning::apt couldn't fetch Ubuntu's package indexes; this try ends here"
  exit 1
fi
