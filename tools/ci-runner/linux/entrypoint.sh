#!/usr/bin/env bash
# Runs one job, then exits (todo X5, README.md). loop.sh on the host starts this container with
# a single-use JIT runner config in ACTIONS_RUNNER_INPUT_JITCONFIG; the runner registers with it,
# takes one job and deregisters, and the container (`docker run --rm`) is thrown away.
#
# The config goes in through that variable, not `run.sh --jitconfig`, so it is never on a
# command line (/proc/<pid>/cmdline is world-readable). The runner reads ACTIONS_RUNNER_INPUT_*
# variables as arguments, masks this one as a secret in its logs and removes it from the
# environment its job steps inherit.
set -euo pipefail

if [ -z "${ACTIONS_RUNNER_INPUT_JITCONFIG:-}" ]; then
  echo "lamina-runner-entrypoint: ACTIONS_RUNNER_INPUT_JITCONFIG is not set (start the runner with up.sh)" >&2
  exit 64
fi
exec /home/runner/run.sh
