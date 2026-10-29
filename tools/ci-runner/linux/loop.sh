#!/usr/bin/env bash
# The host-side supervisor of the Linux CI runner (todo X5, README.md). up.sh installs a copy of
# it as a launchd agent; you can also run it directly in a terminal. Forever: wait for Docker,
# remove a stale registration left by a crash or reboot, mint a single-use JIT runner config with
# the owner's `gh` login, and run one ephemeral runner container with it (`docker run --rm`), so
# every job starts from a clean container. The config is passed through the environment, never
# written to disk or printed; no long-lived credential enters the container.
set -euo pipefail

REPO=${LAMINA_RUNNER_REPO:-laudebugs/lamina}
NAME=${LAMINA_RUNNER_NAME:-lamina-self-hosted-linux-1}
IMAGE=${LAMINA_RUNNER_IMAGE:-lamina-ci-runner-linux:local}

log() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }

child=
stop() {
  log "stopping"
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  if [ -n "$child" ]; then wait "$child" 2>/dev/null || true; fi
  exit 0
}
trap stop INT TERM

# Deletes the repository runner called $NAME, if one is registered. Ephemeral runners remove
# themselves after their job; an idle one killed by a reboot stays behind (offline) and would
# make the next registration under the same name fail.
remove_stale() {
  local id
  id=$(gh api "repos/$REPO/actions/runners?per_page=100" \
    -q ".runners[] | select(.name == \"$NAME\") | .id") || return 1
  if [ -n "$id" ]; then
    log "removing the stale registration of $NAME (id $id)"
    gh api -X DELETE "repos/$REPO/actions/runners/$id" >/dev/null
  fi
}

while true; do
  if ! docker info >/dev/null 2>&1; then
    log "Docker is not running; retrying in 30 s"
    sleep 30
    continue
  fi
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  if ! remove_stale; then
    log "could not list the repository's runners (is gh logged in?); retrying in 60 s"
    sleep 60
    continue
  fi
  # runner_group_id 1 is the default group, the only one a repository has.
  if ! LAMINA_RUNNER_JITCONFIG=$(gh api -X POST "repos/$REPO/actions/runners/generate-jitconfig" \
    -f name="$NAME" -F runner_group_id=1 -f work_folder=_work \
    -f 'labels[]=self-hosted' -f 'labels[]=Linux' -f 'labels[]=ARM64' \
    -q .encoded_jit_config); then
    log "could not create a JIT runner config; retrying in 60 s"
    sleep 60
    continue
  fi
  export LAMINA_RUNNER_JITCONFIG
  log "starting $NAME"
  started=$(date +%s)
  # `-e NAME` without a value copies it from this environment, so it isn't on a command line.
  docker run --rm --name "$NAME" --platform linux/arm64 \
    --cpus 3 --memory 6g \
    -v lamina-linux-cargo:/home/runner/.cargo \
    -v lamina-linux-rustup:/home/runner/.rustup \
    -e LAMINA_RUNNER_JITCONFIG \
    "$IMAGE" &
  child=$!
  unset LAMINA_RUNNER_JITCONFIG
  status=0
  wait "$child" || status=$?
  child=
  log "$NAME exited with status $status"
  # A runner that dies at once (bad image, network) shouldn't spin.
  if [ $(($(date +%s) - started)) -lt 30 ]; then sleep 30; fi
done
