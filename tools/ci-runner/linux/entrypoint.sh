#!/usr/bin/env bash
# Runs one job, then exits (todo X5, README.md). loop.sh on the host starts this container with
# a single-use JIT runner config in LAMINA_RUNNER_JITCONFIG; the runner registers with it, takes
# one job and deregisters, and the container (`docker run --rm`) is thrown away.
set -euo pipefail

if [ -z "${LAMINA_RUNNER_JITCONFIG:-}" ]; then
  echo "lamina-runner-entrypoint: LAMINA_RUNNER_JITCONFIG is not set (start the runner with up.sh)" >&2
  exit 64
fi
jitconfig=$LAMINA_RUNNER_JITCONFIG
# Keep the config out of the environment the job's steps inherit.
unset LAMINA_RUNNER_JITCONFIG
exec /home/runner/run.sh --jitconfig "$jitconfig"
