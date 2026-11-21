# The store build's memory-limit test failed when the main process's watchdog stopped the engine first

[Resolution](resolution.md).

## What we saw

On 2026-11-21 the Nightly (full budgets) run [44839297888](https://github.com/laudebugs/laminapdf/actions/runs/44839297888/job/136654126043) failed on macOS x64, at `cargo xtask check rust`. One test of 6,481 failed, `laminapdf-desktop-engine::sandbox_selftest store_build::memory_past_the_limit_should_stop_the_engine`, after 2.3 s:

```text
exit: memory limit: the watchdog stopped it at 259 MiB (limit 256 MiB)
None (memory limit: the watchdog stopped it at 259 MiB (limit 256 MiB)): rendering is off: render-backend-unavailable
```

The test expects the engine to stop itself and say so on stderr ("the engine stopped itself"). The engine did nothing wrong: the main process's watchdog killed it first. The nightly before it (run 44775901851, on 155b7219) succeeded, and the commits between the two change no desktop code.

## Cause

In the Mac App Store build only the engine watches its memory: App Sandbox doesn't let the main process read the engine's footprint, so `spawn_sandboxed` starts no watchdog there (`in_app_sandbox()`). The test models that build by setting `APP_SANDBOX_CONTAINER_ID` on the engine's command, but `spawn_sandboxed` reads the same variable in its own process, the test's, where it is unset. So the test ran both checks: the engine's, every 250 ms, and the watchdog's, every second.

The probe takes memory in 64 MiB steps. The fourth step crosses the 256 MiB limit at once (259 MiB with the engine's own pages), and the first check to read the footprint after that ends the run. The engine's check is four times as frequent, but the watchdog's next read can still come first. The test's comment said the engine "is first here too", which holds on most runs and not on all.

## How to reproduce

Not on demand: it depends on where the two timers fall. The test only runs on macOS. The two periods and the probe's 64 MiB steps are in `crates/laminapdf-desktop-sandbox/src/spawn/macos.rs`, `src/lockdown/macos.rs` and `src/probe.rs`.
