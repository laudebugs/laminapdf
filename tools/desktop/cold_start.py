"""Launch the desktop app again and again and report its cold start (D2g, ADR 0008 §4).

The app must be built with the `e2e` feature: its first command then logs the time it reached
`run`, began `setup`, built its window and received that command. This script adds the time from
launch to `run` and prints each run and the medians, in milliseconds.

    python3 tools/desktop/cold_start.py target/release/laminapdf-desktop [runs] [other-app]

With a second app, the two are launched in turn, so a busy machine slows both alike. Standard
library only; run by hand, never by tests.
"""

import re
import statistics
import subprocess
import sys
import time

TIMINGS = re.compile(
    r"start_unix_ms=(\d+).*setup_ms=Some\(([0-9.]+)\).*window_ms=Some\(([0-9.]+)\)"
    r".*first_command_ms=Some\(([0-9.]+)\)"
)
STEPS = ("launch → run", "run → setup", "setup → window", "window → first command", "launch → first command")


def launch(app):
    """One launch: the milliseconds of each step, or None when the app logged no timings."""
    launched = time.time() * 1000
    process = subprocess.Popen([app], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    deadline = time.time() + 20
    found = None
    for line in process.stdout:
        found = TIMINGS.search(line)
        if found or time.time() > deadline:
            break
    process.kill()
    process.wait()
    if not found:
        return None
    start, setup, window, command = (float(found.group(k)) for k in range(1, 5))
    to_run = start - launched
    return (to_run, setup, window - setup, command - window, to_run + command)


def main():
    apps = [sys.argv[1]] + sys.argv[3:4]
    runs = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    results = {app: [] for app in apps}
    for i in range(runs):
        for app in apps:
            steps = launch(app)
            if steps is None:
                print(f"{app} run {i}: no timings (was it built with the e2e feature?)")
            else:
                results[app].append(steps)
                print(f"{app} run {i}: " + "  ".join(f"{s} {v:.0f}" for s, v in zip(STEPS, steps)))
            time.sleep(1)
    for app, rows in results.items():
        if rows:
            print(f"\n{app}, medians of {len(rows)} runs:")
            for k, step in enumerate(STEPS):
                print(f"  {step:24} {statistics.median(r[k] for r in rows):6.0f}")


if __name__ == "__main__":
    main()
