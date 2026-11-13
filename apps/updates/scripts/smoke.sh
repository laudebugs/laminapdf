#!/usr/bin/env bash
# Smoke test of a deployed update Worker: /healthz answers 200, an unknown channel 404, a bad arch
# 400, and a real channel 200 or 204 with no-store. Retries for a minute while a new deploy's custom
# domain settles. Usage: scripts/smoke.sh https://updates.laminapdf.com
set -euo pipefail
base=${1:?usage: smoke.sh <base URL>}

status() {
  curl -sS -o /dev/null -w '%{http_code}' --max-time 10 "$base$1" || echo 000
}

for attempt in $(seq 1 12); do
  [ "$(status /healthz)" = 200 ] && break
  [ "$attempt" = 12 ] && { echo "::error::$base/healthz never answered 200"; exit 1; }
  sleep 5
done

fail=0
check() {
  local got
  got=$(status "$1")
  if ! [[ " $2 " == *" $got "* ]]; then
    echo "::error::$base$1 answered $got, expected one of: $2"
    fail=1
  fi
}
check /canary/darwin/aarch64/0.0.1 404
check /stable/darwin/arm64/0.0.1 400
check /stable/darwin/aarch64/0.0.1 "200 204"
check "/staging/darwin/aarch64/0.0.1" "200 204"

cache=$(curl -sS -D - -o /dev/null --max-time 10 "$base/stable/darwin/aarch64/0.0.1" | tr -d '\r' | grep -i '^cache-control:' || true)
if [ "${cache,,}" != "cache-control: no-store" ]; then
  echo "::error::$base answered with '${cache:-no Cache-Control}', expected no-store"
  fail=1
fi
exit "$fail"
