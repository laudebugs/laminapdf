#!/usr/bin/env bash
# Gives each apt download 30 s and 3 tries, rather than waiting on a stalled mirror until the
# try's `sudo timeout` (docs/bugs/ci-apt-retry-kill-eperm). Writing it again changes nothing.
set -euo pipefail
printf '%s\n' 'Acquire::http::Timeout "30";' 'Acquire::https::Timeout "30";' 'Acquire::Retries "3";' \
  | sudo tee /etc/apt/apt.conf.d/99-fail-fast > /dev/null
