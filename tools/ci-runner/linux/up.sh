#!/usr/bin/env bash
# Starts lamina's Linux (ARM64) CI runner on this Mac (todo X5, README.md): installs loop.sh and
# common.sh as a launchd agent (it starts at login and restarts if it dies) and starts it. The
# agent builds the image from main (several minutes the first time) and then runs one fresh
# runner container per job. Needs Docker Desktop running and `gh` logged in as a repository
# admin. Safe to run again: it replaces the agent and its container. Nothing secret is written
# anywhere; the agent mints a single-use runner config with `gh` for every job.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=SCRIPTDIR/common.sh
. "$here/common.sh"

die() { echo "up.sh: $*" >&2; exit 1; }

[ "$(uname -s)" = Darwin ] || die "this script sets up a launchd agent on macOS; elsewhere run loop.sh under your init system (README.md)"
command -v docker >/dev/null || die "docker not found (install Docker Desktop)"
command -v gh >/dev/null || die "gh not found (brew install gh)"
docker info >/dev/null 2>&1 || die "Docker is not running (start Docker Desktop)"
gh auth status >/dev/null 2>&1 || die "gh is not logged in (gh auth login)"
# generate-jitconfig and the heartbeat variable need admin rights on the repository.
[ "$(gh api "repos/$REPO" -q .permissions.admin)" = true ] || die "your gh login is not an admin of $REPO"

echo "Stopping any previous runner"
domain="gui/$(id -u)"
launchctl bootout "$domain/$LABEL" 2>/dev/null || true
# bootout returns before the service is gone; bootstrapping it again too early fails.
for _ in $(seq 1 30); do
  launchctl print "$domain/$LABEL" >/dev/null 2>&1 || break
  sleep 1
done
launchctl print "$domain/$LABEL" >/dev/null 2>&1 && die "the old agent $LABEL is still loaded; try again in a minute"
docker rm -f "$NAME" >/dev/null 2>&1 || true

# A copy, so the agent doesn't depend on this worktree staying put.
mkdir -p "$SUPPORT_DIR" "$(dirname "$PLIST")" "$(dirname "$LOG_FILE")"
install -m 0755 "$here/loop.sh" "$SUPPORT_DIR/loop.sh"
install -m 0644 "$here/common.sh" "$SUPPORT_DIR/common.sh"

# launchd starts agents with a bare environment: give it the directories of docker and gh, and
# the settings from common.sh (with any LAMINA_RUNNER_* overrides this run was given).
path="$(dirname "$(command -v docker)"):$(dirname "$(command -v gh)"):/usr/bin:/bin:/usr/sbin:/sbin"
cat >"$PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array><string>/bin/bash</string><string>$SUPPORT_DIR/loop.sh</string></array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key><string>$path</string>
    <key>LAMINA_RUNNER_REPO</key><string>$REPO</string>
    <key>LAMINA_RUNNER_NAME</key><string>$NAME</string>
    <key>LAMINA_RUNNER_IMAGE</key><string>$IMAGE</string>
    <key>LAMINA_RUNNER_AGENT</key><string>$LABEL</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>$LOG_FILE</string>
  <key>StandardErrorPath</key><string>$LOG_FILE</string>
</dict>
</plist>
PLIST

launchctl bootstrap "$domain" "$PLIST"
echo "Started. The first start builds the image from main; then $NAME (self-hosted, Linux, ARM64) comes online:"
echo "  gh api repos/$REPO/actions/runners -q '.runners[] | [.name, .status] | @tsv'"
echo "Log: $LOG_FILE"
