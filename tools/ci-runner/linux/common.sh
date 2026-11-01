# Shared settings and helpers for up.sh, down.sh and loop.sh (todo X5, README.md). Sourced, not
# run. The names can be overridden through the environment (LAMINA_PDF_RUNNER_*); up.sh writes the
# values it used into the launchd agent, so loop.sh sees the same ones (run down.sh with them too).
# shellcheck shell=bash disable=SC2034  # the variables are used by the scripts that source this

REPO=${LAMINA_PDF_RUNNER_REPO:-laudebugs/laminapdf}
NAME=${LAMINA_PDF_RUNNER_NAME:-lamina-self-hosted-linux-1}
IMAGE=${LAMINA_PDF_RUNNER_IMAGE:-lamina-ci-runner-linux:local}
LABEL=${LAMINA_PDF_RUNNER_AGENT:-dev.lamina.ci-runner-linux}
# The repository variable loop.sh refreshes while the runner container is up; full.yml skips
# check-linux when it is older than 15 minutes.
HEARTBEAT_VAR=LINUX_RUNNER_HEARTBEAT
SUPPORT_DIR="$HOME/Library/Application Support/lamina-ci-runner-linux"
LOG_FILE="$HOME/Library/Logs/lamina-ci-runner-linux.log"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
# Files the image is built from, as paths in the repository.
IMAGE_DIR=tools/ci-runner/linux
TOOLCHAIN_FILE=rust-toolchain.toml

log() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }

# Prints the id of the repository runner called $NAME (nothing if there is none).
runner_id() {
  gh api "repos/$REPO/actions/runners?per_page=100" \
    -q ".runners[] | select(.name == \"$NAME\") | .id"
}

# Removes the registration of $NAME from GitHub, if there is one. A runner that was just killed
# can still count as busy for a little while (the API answers 422), so this retries for about a
# minute. Returns 1 if the registration is still there.
deregister() {
  local id attempt
  for attempt in 1 2 3 4 5 6 7; do
    id=$(runner_id) || return 1
    [ -n "$id" ] || return 0
    if gh api -X DELETE "repos/$REPO/actions/runners/$id" >/dev/null 2>&1; then
      log "removed the registration of $NAME (id $id)"
      return 0
    fi
    [ "$attempt" -lt 7 ] && sleep 10
  done
  log "could not remove the registration of $NAME (id $id): GitHub still reports it busy"
  return 1
}
