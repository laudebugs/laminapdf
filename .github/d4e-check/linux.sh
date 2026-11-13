#!/usr/bin/env bash
# Temporary (D4e manual check on a hosted runner; deleted before merge): real X11 mouse drags of a
# tab, out of its window and onto another, under Xvfb with a window manager.
set -uo pipefail
out=target/d4e-check; mkdir -p "$out"
export DISPLAY=:99 WEBKIT_DISABLE_DMABUF_RENDERER=1
Xvfb :99 -screen 0 1600x1000x24 & sleep 2
openbox & sleep 1
ex="$PWD/fixtures/golden/examples"
dbus-run-session -- target/debug/laminapdf-desktop "$ex/edit_bookmarks.pdf" "$ex/edit_metadata.pdf" > "$out/app.log" 2>&1 &
sleep 20
shot() { import -window root "$out/$1.png"; }
windows() { for w in $(xdotool search --name "Lamina PDF" 2>/dev/null); do echo "$w $(xdotool getwindowname "$w") $(xdotool getwindowgeometry "$w" | tr '\n' ' ')"; done; }
# xdotool's geometry is off under a reparenting window manager; xwininfo's absolute corner isn't.
geom() { xwininfo -id "$1" | awk '/Absolute upper-left X/ {print "X=" $NF} /Absolute upper-left Y/ {print "Y=" $NF}'; }
drag() { # x1 y1 x2 y2
  xdotool mousemove "$1" "$2"; sleep 0.3; xdotool mousedown 1; sleep 0.3
  for i in $(seq 1 30); do xdotool mousemove $(( $1 + ($3 - $1) * i / 30 )) $(( $2 + ($4 - $2) * i / 30 )); sleep 0.04; done
  sleep "${5:-0.8}"; shot "${6:-mid}"; xdotool mouseup 1; sleep 3
}
echo "## start"; windows; shot 1-start
main=$(xdotool search --name "Lamina PDF" | head -1)
xdotool windowsize --sync "$main" 900 650; xdotool windowmove --sync "$main" 0 0; sleep 1
eval "$(geom "$main")"
echo "main at $X,$Y"
# The shown tab is the second; the strip is the webview's first ~30 px (no title bar inside X/Y).
drag $((X + 301)) $((Y + 16)) 1200 200 0.8 2-mid-out
echo "## after drag out"; windows; shot 3-after-out
new=$(xdotool search --name "edit_metadata.pdf" | head -1)
eval "$(geom "$main")"; mx=$X; my=$Y
xdotool windowactivate --sync "$main" 2>/dev/null; sleep 1
eval "$(geom "$new")"; nx=$X; ny=$Y
echo "new window at $nx,$ny"
# The main window's one tab, dropped on the new window's middle.
drag $((mx + 108)) $((my + 16)) $((nx + 300)) $((ny + 300)) 1.2 4-mid-onto
echo "## after drag onto"; windows; shot 5-after-onto
grep -iv tile "$out/app.log" | tail -20
