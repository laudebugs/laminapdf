# Resolution: the store build's memory-limit test failed when the main process's watchdog stopped the engine first

[Diagnosis](detail.md).

## The fix

The test now starts the engine as the Mac App Store build does, with the engine's check as the only one. `store_selftest_without_watchdog` in `crates/laminapdf-desktop-engine/tests/sandbox_selftest.rs` runs the engine binary with the arguments `spawn_sandboxed` gives it (`--sandbox-selftest=memory`, `--memory-limit`, and `APP_SANDBOX_CONTAINER_ID` in its environment) and no watchdog. The assertion is the same: no outcome, and "the engine stopped itself" on stderr. The engine, the watchdog and the lock-down are unchanged.

The fix is to the test and not to either check, because production never runs both: the main process in App Sandbox starts no watchdog. Making the engine's check faster would only make the race less likely. Accepting a stop by either check would pass even if the engine's own check were broken, which is the thing the test is for.

## How it was checked

- **Compiled on Linux.** The test is macOS-only, so with its `cfg(target_os = "macos")` gates temporarily widened to `unix`, `cargo check -p laminapdf-desktop-engine --test sandbox_selftest --all-features` built without a warning. The gates are back as they were.
- **Not run.** This session has no Mac, so the changed test hasn't run. It runs on the macOS legs of this PR's CI and of every nightly. With no watchdog running, the first check to cross is the engine's by construction, so one pass doesn't prove the fix, but a failure would be a different bug.

## Left for later

- **`spawn_sandboxed` skipping the watchdog in App Sandbox isn't tested.** That is the `in_app_sandbox()` early return in `spawn/macos.rs`. Testing it would need the variable set in the test's own process, and `std::env::set_var` is unsound beside other threads under plain `cargo test`.
- **The other store-build tests still use `spawn_sandboxed`,** so they run the watchdog too. They test what the engine is blocked from, and the watchdog doesn't affect them.
