# D7d — Native crash minidumps, processed on the computer, and debug symbols

| Field | Value |
| --- | --- |
| ID | `D7d` |
| Status | `done` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6b](./D6b-release-workflow-and-linux-packages.md), [D7a](./D7a-crash-reporting.md) |
| ADRs | Follows [ADR 0014 §10](../../decisions/0014-desktop-distribution.md#10-crash-reports) (crash reports), ADR 0008 §5 (the engine's sandbox) |
| Branch | `feat/d7d-native-crash-minidumps` |
| PR title | `feat(desktop): report native crashes from minidumps processed on the computer` |
| Size | L (600–1,000 lines across the app, the engine, xtask and the release workflow) |
| PR | [#655](https://github.com/laudebugs/laminapdf/pull/655) |

## Goal

D7a reports panics and how an engine ended, but a native crash (a segmentation fault in PDFium, an access violation in the main process) leaves no stack. This todo captures a minidump of each native crash, turns it into D7a's report fields on the computer and deletes it, so a native crash's report names the frames that crashed. The release workflow archives debug symbols, and `cargo xtask crash symbolicate` reads a report's frames with them. D7a split this part off: it needs D6b's release workflow, and capture across the engine's sandbox needs its own testing on each OS.

## Context

- [D7a](../done/D7a-crash-reporting.md): the report, its schema (`apps/updates/schemas/crash-report.schema.json`: `modules` and native frames wait for this todo), the scrubber, consent and upload. Read its "As built" notes.
- Crates (licence-check each): `crash-handler` and `minidumper` (Embark, MIT OR Apache-2.0) for out-of-process capture; `minidump`, `minidump-processor` and `minidump-unwind` (rust-minidump, MIT) to process a dump locally.
- **Privacy:** a raw minidump holds stack and possibly heap memory, so possibly document bytes and paths. It is never uploaded; only the processed, allow-listed fields are.
- The engine's sandbox (D2c) may block the socket the crash handler needs; connect before the lock-down.

## Scope

**In scope**

- The main process as the `minidumper` server for its engines (it is unsandboxed, so it writes the dumps); `crash-handler` in the engine, connected before the sandbox goes on; the main process's own native crashes through a small monitor process or the platform's handler.
- Processing each dump with `minidump-processor` into `{ crash_reason, frames: [{ module, offset, function? }], modules: [{ name, version, debug_id }] }`, module names without paths; the dump deleted once processing ends, whether it succeeds or fails.
- The supervisor's `Crashed` event carrying the processed dump, merged with D7a's report.
- D6b's workflow: debug symbols (dSYM, PDB, split DWARF via `objcopy --only-keep-debug`) archived to a **private** R2 bucket `lamina-pdf-symbols`, keyed by debug ID.
- `cargo xtask crash symbolicate <report.json>`: fetches the symbols and prints the stack with names and lines, for maintainers.

**Out of scope**

- The crash question's tab choice: [D7e](../ready/D7e-crash-recovered-tab-choice.md).

## Tasks

- [x] Fresh worktree; claim via the work PR. Load `tauri-v2` and `rust-best-practices`.
- [x] Capture: `crash-handler` in the engine, connected before lock-down; the `minidumper` server in the main process; the main process's own capture.
- [x] Local processing and deletion; frames and modules into D7a's report.
- [x] D6b workflow: symbols to the private bucket; `cargo xtask crash symbolicate`.
- [x] `docs/guidelines/desktop-release.md`: the symbols archive and symbolication.

## Acceptance criteria

- [x] An engine segmentation fault (`e2e_crash_engine_segv` raising a real fault, not a signal from outside) produces a report whose frames name the faulting module, on macOS and Linux. Windows moved to [D7f](../ongoing/D7f-windows-native-crash-capture.md) (see As built).
- [x] Raw minidumps are deleted after processing, also when processing fails (test).
- [x] A Rust integration test: an engine child crashing under the `minidumper` server gives a processed summary with no marker of the open file's folder, name or content.
- [x] The engine's sandbox selftest still passes with the crash handler connected.
- [x] Engine crashes still trigger the D2b restart flow.

## Post-merge checks

- [ ] The first tagged release archives its symbols, and `cargo xtask crash symbolicate` reads a report from it.

## Tests

Rust unit tests for the processing (a committed small minidump fixture of each OS), the integration test above, and the e2e crash run (D7a's `wdio.crash.conf.ts`) extended with a native fault.

## Docs to update

ADR 0014 §10 ("What comes later" becomes as built), the desktop plan's §9 D7 to-do, `docs/guidelines/desktop-release.md`.

## Risks and open questions

- The sandbox may block the handler's connection on some OS; connect before lock-down and test each OS.
- A dump's processing needs memory and time: cap both, and drop the dump past them.

## As built

These differ from the text above:

- **Windows has no capture yet.** minidumper's Windows server takes the process ID its client sends without checking it, so a compromised engine could have the server dump any of the user's processes. Windows keeps D7a's reports; [D7f](../ongoing/D7f-windows-native-crash-capture.md) turns capture on once the server checks its peer.
- **Nothing hostile in the main process.** A monitor process, the app's binary started with `--crash-monitor`, hosts both dump servers (the main process's and the engines'), so a hostile message can end only the monitor, which the app restarts at most 5 times a session; inside the monitor, a broken server loop restarts at most 16 times a minute. A dump is read by a fresh engine in `--summarize-dump` mode under the full sandbox, killed after 10 s, its answer capped at 1 MiB; the main process links no minidump parser (`full.yml` checks).
- **Its own crate.** Capture and reading live in the edge crate `laminapdf-desktop-crash`; the sandbox crate keeps only the seccomp allowance for the crash socket (and `PR_SET_PTRACER` naming the dump server alone).
- **The report's shape.** The data is a new `native` object (`crash_reason`, frames as module and offset, modules with debug IDs), not D7a's reserved top-level `modules`, which stays empty. `function` is always null: a name read from memory could carry document text, so names come only from the archived symbols through `cargo xtask crash symbolicate`.
- **Joined by process ID.** `Crashed` carries the engine's pid, and the app waits for that pid's summary, for a bounded time. On Linux the pid is the status stream's `Tgid`, since its `Pid` and the processed state's process ID are the crashing thread's.
- **Limits.** Each dump server refuses a new dump while its folder holds 4, or after 6 in a minute; dumps are written as `.tmp` and renamed when complete; leftover main-process dumps are read off the start-up path, the newest 3.
- **Size.** The dump reader is rust-minidump's `minidump` and `minidump-unwind` without the processor or a symbol parser, its crates built for size; the installers still grew about 0.6 MB, and the owner raised the budget from 15 to 16 MB (2026-11-15).
- **Symbolicate.** It prints a report's strings without control characters and reaches the bucket only with `--fetch`.
- **Fixtures.** One minidump fixture, macOS Arm64; Linux's capture is proven by CI's tests, and Windows' waits for D7f.
