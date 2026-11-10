# Desktop engine tests fail now and then right after a restart

[Resolution](resolution.md).

## What we saw

On PR #407's CI (2026-11-10), the Linux ARM64 Rust job failed `laminapdf-desktop-engine::engine_process the_password_should_unlock_the_document_again_in_a_restarted_engine`: its first request after the engine restarted unwrapped `SupervisorError::Lost`. The PR changed nothing in the desktop crates, and the test passed on every other runner and on a rerun.

## How to reproduce

It needs the runner to be slow at one moment, so it doesn't reproduce on demand. Three tests share the pattern: the engine test above, the supervisor's `a_request_after_a_restart_should_reach_the_new_engine`, and the app's `pages_should_work_again_after_a_restart`.

## Cause

D2b's restoring gate (ADR 0008 §2) turns away requests from other threads while a restarted engine's documents are reopened, until the `Restarted` handler returns. The tests' handlers send the event to the test thread from inside the handler, so the test can ask before the handler has returned and the gate has opened. The gate is right; the tests asked too early.
