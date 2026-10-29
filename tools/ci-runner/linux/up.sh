#!/usr/bin/env bash
# Starts lamina's Linux (ARM64) CI runner on this Mac (todo X5, README.md): builds the image,
# installs loop.sh as a launchd agent (it starts at login and restarts if it dies), and starts it.
# Needs Docker Desktop running and `gh` logged in as a repository admin. Safe to run again: it
# replaces the agent, the container and the image. Nothing secret is written anywhere; the
# agent mints a single-use runner config with `gh` for every job.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
LABEL=dev.lamina.ci-runner-linux
NAME=lamina-self-hosted-linux-1
IMAGE=lamina-ci-runner-linux:local
REPO=laudebugs/lamina
support="$HOME/Library/Application Support/lamina-ci-runner-linux"
logs="$HOME/Library/Logs/lamina-ci-runner-linux.log"
plist="$HOME/Library/LaunchAgents/$LABEL.plist"

die() { echo "up.sh: $*" >&2; exit 1; }

[ "$(uname -s)" = Darwin ] || die "this script sets up a launchd agent on macOS; elsewhere run loop.sh under your init system (README.md)"
command -v docker >/dev/null || die "docker not found (install Docker Desktop)"
command -v gh >/dev/null || die "gh not found (brew install gh)"
docker info >/dev/null 2>&1 || die "Docker is not running (start Docker Desktop)"
gh auth status >/dev/null 2>&1 || die "gh is not logged in (gh auth login)"
# generate-jitconfig needs admin rights on the repository.
[ "$(gh api "repos/$REPO" -q .permissions.admin)" = true ] || die "your gh login is not an admin of $REPO"

echo "Building $IMAGE"
docker build --platform linux/arm64 -t "$IMAGE" "$here"

echo "Stopping any previous runner"
launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
docker rm -f "$NAME" >/dev/null 2>&1 || true

# A copy, so the agent doesn't depend on this worktree staying put.
mkdir -p "$support" "$(dirname "$plist")" "$(dirname "$logs")"
install -m 0755 "$here/loop.sh" "$support/loop.sh"

# launchd starts agents with a bare PATH; give it the directories of docker and gh.
path="$(dirname "$(command -v docker)"):$(dirname "$(command -v gh)"):/usr/bin:/bin:/usr/sbin:/sbin"
cat >"$plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array><string>/bin/bash</string><string>$support/loop.sh</string></array>
  <key>EnvironmentVariables</key>
  <dict><key>PATH</key><string>$path</string></dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>$logs</string>
  <key>StandardErrorPath</key><string>$logs</string>
</dict>
</plist>
PLIST

launchctl bootstrap "gui/$(id -u)" "$plist"
echo "Started. The runner appears as $NAME (self-hosted, Linux, ARM64) within a minute:"
echo "  gh api repos/$REPO/actions/runners -q '.runners[] | [.name, .status] | @tsv'"
echo "Log: $logs"
