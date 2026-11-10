#!/usr/bin/env bash
# The machine and webview a run was made on (macOS and Linux), for the README's environment table.
set -uo pipefail
echo "date: $(date -u +%Y-%m-%dT%H:%MZ)"
echo "uname: $(uname -srm)"
echo "rustc: $(rustc --version)"
echo "tauri: $(grep -A1 '^name = "tauri"$' "$(dirname "$0")/../src-tauri/Cargo.lock" | tail -1)"
if [ "$(uname)" = Darwin ]; then
  echo "os: macOS $(sw_vers -productVersion) ($(sw_vers -buildVersion)); WKWebView ships with the OS"
  echo "cpu: $(sysctl -n machdep.cpu.brand_string), $(sysctl -n hw.ncpu) cores"
  echo "memory: $(( $(sysctl -n hw.memsize) / 1073741824 )) GiB"
  system_profiler SPDisplaysDataType 2>/dev/null | grep -E 'Chipset Model|Resolution|Retina|Cores' | sed 's/^ */gpu\/display: /'
else
  echo "os: $(. /etc/os-release && echo "$PRETTY_NAME")"
  echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs), $(nproc) cores"
  echo "memory: $(free -g | awk '/Mem:/ {print $2}') GiB"
  echo "webkit2gtk-4.1: $(pkg-config --modversion webkit2gtk-4.1 2>/dev/null || echo unknown)"
  echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-} GDK_BACKEND=${GDK_BACKEND:-} DISPLAY=${DISPLAY:-} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-}"
  echo "WEBKIT_DISABLE_DMABUF_RENDERER=${WEBKIT_DISABLE_DMABUF_RENDERER:-unset}"
  echo "gl: $(glxinfo -B 2>/dev/null | grep -E 'OpenGL renderer' | cut -d: -f2 | xargs || echo unknown)"
fi
exit 0
