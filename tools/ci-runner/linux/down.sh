#!/usr/bin/env bash
# Stops lamina's Linux CI runner (todo X5, README.md): removes the launchd agent, the container
# and the runner's registration on GitHub. A job running on it is cancelled. `--prune` also
# deletes the image and the Rust volumes (Docker Desktop's disk image doesn't shrink otherwise).
set -euo pipefail

LABEL=dev.lamina.ci-runner-linux
NAME=lamina-self-hosted-linux-1
IMAGE=lamina-ci-runner-linux:local
REPO=laudebugs/lamina
support="$HOME/Library/Application Support/lamina-ci-runner-linux"
plist="$HOME/Library/LaunchAgents/$LABEL.plist"

prune=false
case "${1:-}" in
  "") ;;
  --prune) prune=true ;;
  *) echo "usage: down.sh [--prune]" >&2; exit 64 ;;
esac

if command -v launchctl >/dev/null; then
  launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
fi
rm -f "$plist"
rm -rf "$support"
docker rm -f "$NAME" >/dev/null 2>&1 || true

id=$(gh api "repos/$REPO/actions/runners?per_page=100" -q ".runners[] | select(.name == \"$NAME\") | .id")
if [ -n "$id" ]; then
  gh api -X DELETE "repos/$REPO/actions/runners/$id" >/dev/null
  echo "Removed $NAME from $REPO"
fi

if $prune; then
  docker image rm "$IMAGE" 2>/dev/null || true
  docker volume rm lamina-linux-cargo lamina-linux-rustup 2>/dev/null || true
  echo "Removed the image and volumes (docker system df shows what is left; docker builder prune drops the build cache)"
fi
echo "Stopped."
