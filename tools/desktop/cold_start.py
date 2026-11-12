"""Launch the desktop app again and again and check its cold-start gates (ADR 0008 §4).

The app must be built with the `e2e` feature: its first command then logs the time it reached
`run`, began `setup`, built and showed its window and received that command (`app_info`, sent once
the empty state has rendered). This script adds the time from launch to `run` and prints each run
and the medians, in milliseconds.

    python3 tools/desktop/cold_start.py <app> [runs] [floor-app]

With a floor build (the app with tools/desktop/floor/ as its frontend), the two are launched in
turn, so a busy machine slows both alike, and the script checks both gates: launch → window under
300 ms, and the app's launch → first command within 50 ms of the floor's. It exits 1 when a gate
fails. Standard library only; run by hand, never by tests.
"""

import queue
import re
import statistics
import subprocess
import sys
import threading
import time

TIMINGS = re.compile(
    r"start_unix_ms=(\d+).*setup_ms=([0-9.]+).*window_ms=([0-9.]+).*first_command_ms=([0-9.]+)"
)
STEPS = (
    "launch → run",
    "run → setup",
    "setup → window",
    "window → first command",
    "launch → window",
    "launch → first command",
)
WINDOW_GATE_MS = 300
OVER_FLOOR_GATE_MS = 50
TIMEOUT_S = 20


def launch(app):
    """One launch: the milliseconds of each step, or None when no timings came within the timeout."""
    launched = time.time() * 1000
    process = subprocess.Popen([app], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines = queue.Queue()
    # A reader thread, so a silent app still times out.
    threading.Thread(target=lambda: [lines.put(line) for line in process.stdout], daemon=True).start()
    deadline = time.time() + TIMEOUT_S
    found = None
    while found is None and time.time() < deadline:
        try:
            found = TIMINGS.search(lines.get(timeout=max(0.0, deadline - time.time())))
        except queue.Empty:
            break
    process.kill()
    process.wait()
    if found is None:
        return None
    start, setup, window, command = (float(found.group(k)) for k in range(1, 5))
    to_run = start - launched
    return (to_run, setup, window - setup, command - window, to_run + window, to_run + command)


def medians(rows):
    return [statistics.median(r[k] for r in rows) for k in range(len(STEPS))]


def main():
    apps = [sys.argv[1]] + sys.argv[3:4]
    runs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    results = {app: [] for app in apps}
    for i in range(runs):
        for app in apps:
            steps = launch(app)
            if steps is None:
                print(f"{app} run {i}: no timings in {TIMEOUT_S} s (was it built with the e2e feature?)")
            else:
                results[app].append(steps)
                print(f"{app} run {i}: " + "  ".join(f"{s} {v:.0f}" for s, v in zip(STEPS, steps)))
            time.sleep(1)
    if any(not rows for rows in results.values()):
        sys.exit("no timings from one of the apps")
    for app, rows in results.items():
        print(f"\n{app}, medians of {len(rows)} runs:")
        for step, value in zip(STEPS, medians(rows)):
            print(f"  {step:24} {value:6.0f}")
    if len(apps) == 2:
        app, floor = (medians(results[a]) for a in apps)
        window, over = app[4], app[5] - floor[5]
        ok = window < WINDOW_GATE_MS and over <= OVER_FLOOR_GATE_MS
        print(f"\nlaunch → window {window:.0f} ms (gate < {WINDOW_GATE_MS})")
        print(f"first command over the floor {over:.0f} ms (gate ≤ {OVER_FLOOR_GATE_MS})")
        print("gates pass" if ok else "gates FAIL")
        sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
