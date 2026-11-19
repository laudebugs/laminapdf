#!/usr/bin/env bash
# Gives each apt download 30 s and 3 tries, rather than waiting on a stalled mirror until the
# try's `sudo timeout` (docs/bugs/ci-apt-retry-kill-eperm). Writing it again changes nothing.
set -euo pipefail
printf '%s\n' 'Acquire::http::Timeout "30";' 'Acquire::https::Timeout "30";' 'Acquire::Retries "3";' \
  | sudo tee /etc/apt/apt.conf.d/99-fail-fast > /dev/null

# noble's runner image names Ubuntu's mirrors in this file (apt's mirror+file method: one URI per
# line, then optional tab-separated `key:value` fields, the lowest `priority` tried first). On
# 2026-11-19 a job fetched the indexes, then every mirror it lists (azure, archive, security)
# refused both .deb downloads (docs/bugs/ci-apt-slow-or-unreachable-mirror). Kernel.org's mirror
# sits behind a CDN, so it rarely fails with them: it goes last, after the image's own entries.
# Ubuntu's main archive only: an Arm64 runner's ports packages aren't on it.
mirrors=/etc/apt/apt-mirrors.txt
fallback=http://mirrors.edge.kernel.org/ubuntu/
if [ -f "$mirrors" ] && [ "$(dpkg --print-architecture)" = amd64 ] && ! grep -qF "$fallback" "$mirrors"; then
  last=$(grep -oE 'priority:[0-9]+' "$mirrors" | cut -d: -f2 | sort -n | tail -n 1 || true)
  if [ -n "$last" ]; then line=$(printf '%s\tpriority:%d' "$fallback" "$((last + 1))"); else line=$fallback; fi
  # A file that doesn't end in a newline would join the fallback to its last line.
  if [ -s "$mirrors" ] && [ -n "$(tail -c 1 "$mirrors")" ]; then echo | sudo tee -a "$mirrors" > /dev/null; fi
  echo "$line" | sudo tee -a "$mirrors" > /dev/null
fi
