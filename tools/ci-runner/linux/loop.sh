#!/usr/bin/env bash
# The host-side supervisor of the Linux CI runner (todo X5, README.md). up.sh installs a copy of
# it (with common.sh) as a launchd agent. Forever:
#   - wait for Docker;
#   - (re)build the image from main's copy of tools/ci-runner/linux and rust-toolchain.toml when
#     either changed on main, keeping the last good image if the build fails;
#   - remove a registration a crash or reboot left behind;
#   - mint a single-use JIT runner config with the owner's `gh` login and run one ephemeral
#     runner container with it (`docker run --rm`, no volumes), so every job starts clean.
# In the background it refreshes the repository variable LINUX_RUNNER_HEARTBEAT every 5 minutes
# while the runner container is up; full.yml skips check-linux when the heartbeat is stale.
# The JIT config goes to Docker through the environment, never on a command line, in a file or
# in the log; no long-lived credential enters the container.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=SCRIPTDIR/common.sh
. "$here/common.sh"

heartbeat() {
  while true; do
    if [ -n "$(docker ps -q --filter "name=^${NAME}\$" 2>/dev/null)" ]; then
      gh variable set "$HEARTBEAT_VAR" --repo "$REPO" --body "$(date +%s)" >/dev/null 2>&1 \
        || log "could not refresh $HEARTBEAT_VAR"
    fi
    sleep 300
  done
}

# The last commits on main that touched the image's inputs, e.g. "<sha>+<sha>".
source_key() {
  local dir toolchain
  dir=$(gh api "repos/$REPO/commits?sha=main&path=$IMAGE_DIR&per_page=1" -q '.[0].sha') || return 1
  toolchain=$(gh api "repos/$REPO/commits?sha=main&path=$TOOLCHAIN_FILE&per_page=1" -q '.[0].sha') || return 1
  [ -n "$dir" ] && [ -n "$toolchain" ] || return 1
  printf '%s+%s' "$dir" "$toolchain"
}

# Prints a repository file at a revision.
fetch() {
  gh api -H 'Accept: application/vnd.github.raw' "repos/$REPO/contents/$1?ref=$2"
}

# Builds $IMAGE from main unless it was already built from the current inputs. Fails only when
# there is no usable image at all.
ensure_image() {
  local key built ctx main
  if ! key=$(source_key); then
    log "could not read main's runner sources"
    docker image inspect "$IMAGE" >/dev/null 2>&1
    return
  fi
  built=$(docker image inspect "$IMAGE" -f '{{ index .Config.Labels "dev.lamina.source" }}' 2>/dev/null || true)
  [ "$key" = "$built" ] && return 0
  log "building $IMAGE from main ($key)"
  ctx=$(mktemp -d)
  local status=0
  if ! { main=$(gh api "repos/$REPO/commits/main" -q .sha) &&
    fetch "$IMAGE_DIR/Dockerfile" "$main" >"$ctx/Dockerfile" &&
    fetch "$IMAGE_DIR/entrypoint.sh" "$main" >"$ctx/entrypoint.sh" &&
    fetch "$TOOLCHAIN_FILE" "$main" >"$ctx/rust-toolchain.toml" &&
    docker build --platform linux/arm64 --label "dev.lamina.source=$key" -t "$IMAGE" "$ctx"; }; then
    status=1
  fi
  rm -rf "$ctx"
  if [ "$status" -ne 0 ]; then
    log "the build from main failed; keeping the previous image"
    docker image inspect "$IMAGE" >/dev/null 2>&1
    return
  fi
  # This supervisor isn't rebuilt with the image: say when main's copy differs from this one.
  local f ours theirs
  for f in loop.sh common.sh; do
    ours=$(git hash-object "$here/$f" 2>/dev/null || true)
    theirs=$(gh api "repos/$REPO/contents/$IMAGE_DIR/$f?ref=$main" -q .sha 2>/dev/null || true)
    if [ -n "$theirs" ] && [ "$ours" != "$theirs" ]; then
      log "$f changed on main: rerun tools/ci-runner/linux/up.sh to update this agent"
    fi
  done
}

beat=
child=
stop() {
  log "stopping"
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  if [ -n "$child" ]; then wait "$child" 2>/dev/null || true; fi
  if [ -n "$beat" ]; then kill "$beat" 2>/dev/null || true; fi
  exit 0
}
trap stop INT TERM

heartbeat &
beat=$!

while true; do
  if ! docker info >/dev/null 2>&1; then
    log "Docker is not running; retrying in 30 s"
    sleep 30
    continue
  fi
  if ! ensure_image; then
    log "no runner image yet; retrying in 60 s"
    sleep 60
    continue
  fi
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  # Ephemeral runners remove themselves after their job; one killed while idle (a reboot, a
  # crash) stays registered and would block a new registration under the same name.
  if ! deregister; then
    log "a registration of $NAME is still in the way (or gh can't reach GitHub); retrying in 60 s"
    sleep 60
    continue
  fi
  # runner_group_id 1 is the default group, the only one a repository has.
  if ! ACTIONS_RUNNER_INPUT_JITCONFIG=$(gh api -X POST "repos/$REPO/actions/runners/generate-jitconfig" \
    -f name="$NAME" -F runner_group_id=1 -f work_folder=_work \
    -f 'labels[]=self-hosted' -f 'labels[]=Linux' -f 'labels[]=ARM64' \
    -q .encoded_jit_config); then
    log "could not create a JIT runner config (is gh logged in?); retrying in 60 s"
    sleep 60
    continue
  fi
  export ACTIONS_RUNNER_INPUT_JITCONFIG
  log "starting $NAME"
  started=$(date +%s)
  # `-e NAME` without a value copies it from this environment, so it isn't on a command line.
  docker run --rm --name "$NAME" --platform linux/arm64 \
    --cpus 3 --memory 6g \
    -e ACTIONS_RUNNER_INPUT_JITCONFIG \
    "$IMAGE" &
  child=$!
  unset ACTIONS_RUNNER_INPUT_JITCONFIG
  status=0
  wait "$child" || status=$?
  child=
  log "$NAME exited with status $status"
  # A runner that dies at once (bad image, network) shouldn't spin.
  if [ $(($(date +%s) - started)) -lt 30 ]; then sleep 30; fi
done
