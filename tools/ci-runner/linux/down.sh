#!/usr/bin/env bash
# Stops lamina's Linux CI runner (todo X5, README.md): removes the launchd agent, the container,
# the runner's registration on GitHub and the heartbeat variable. A job running on it is
# cancelled. `--prune` also deletes the image (Docker Desktop's disk image doesn't shrink
# otherwise).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=SCRIPTDIR/common.sh
. "$here/common.sh"

prune=false
case "${1:-}" in
  "") ;;
  --prune) prune=true ;;
  *) echo "usage: down.sh [--prune]" >&2; exit 64 ;;
esac

if command -v launchctl >/dev/null; then
  launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
fi
rm -f "$PLIST"
rm -rf "$SUPPORT_DIR"
docker rm -f "$NAME" >/dev/null 2>&1 || true

# Without a heartbeat, full.yml stops scheduling check-linux within 15 minutes.
gh variable delete "$HEARTBEAT_VAR" --repo "$REPO" >/dev/null 2>&1 || true
if ! deregister; then
  echo "down.sh: warning: $NAME is still registered; delete it under Settings > Actions > Runners" >&2
fi

if $prune; then
  docker image rm "$IMAGE" 2>/dev/null || true
  echo "Removed the image (docker system df shows what is left; docker builder prune drops the build cache)"
fi
echo "Stopped."
