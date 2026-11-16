# D7f — Native crash capture on Windows, with the dump server checking who asks

| Field | Value |
| --- | --- |
| ID | `D7f` |
| Status | `done` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D7d](./D7d-native-crash-minidumps-and-symbols.md) |
| ADRs | Follows [ADR 0014 §10](../../decisions/0014-desktop-distribution.md#10-crash-reports) (crash reports), ADR 0008 §5 (the engine's sandbox: it can't touch other processes) |
| Branch | `feat/d7f-windows-native-crash-capture` |
| PR title | `feat(desktop): capture native crashes on Windows, dumping only the process that asks` |
| Size | M (200–500 lines, most of it in the dump server's Windows path) |
| PR | [#674](https://github.com/laudebugs/laminapdf/pull/674) |

## Goal

D7d captures native crashes on macOS and Linux but leaves Windows out: there, minidumper's server takes the process ID the client sends and dumps that process without checking who sent it. A compromised engine could make the dump server open and dump any process of the user, the main process with its open documents or a password manager among them. This todo turns Windows capture on once the server dumps only the process on the other end of its connection, and only a process the app started.

## Context

- [D7d](../done/D7d-native-crash-minidumps-and-symbols.md) and its review on #655: the finding, and the design D7d shipped (the monitor process hosts the dump server; a sandboxed engine process reads each dump).
- minidumper's Windows server (`server.rs`) and minidump-writer's `windows/minidump_writer.rs`: where the client's `process_id` becomes `OpenProcess(PROCESS_ALL_ACCESS, pid)`.
- Windows' AF_UNIX sockets name their peer with the `SIO_AF_UNIX_GETPEERPID` ioctl.

## Scope

**In scope**

- The server checks the connection's peer process ID (`SIO_AF_UNIX_GETPEERPID`) against the dump request's, and against the engines the supervisor started (or the main process, for its own socket), before it opens any process. Upstream to minidumper if the maintainers take it; until then a pinned fork, with its licence and the reason in `deny.toml`'s sources.
- The engine's Low-integrity, restricted token connecting to the monitor's socket: a socket only the engines need, labelled for them alone; the main process's own socket unlabelled.
- Windows in D7d's tests: the engine's fault under the dump server, the marker test, the selftest with the handler connected, and the e2e crash run expecting `native` on Windows too.

**Out of scope**

- macOS and Linux, which D7d covers.

## Tasks

- [x] Fresh worktree; claim via the work PR. Load `tauri-v2` and `rust-best-practices`.
- [x] The peer check in the dump server, upstream or in a pinned fork; a test that a client asking for another process's dump gets nothing and the server keeps serving.
- [x] Windows capture on: the engine's client before lock-down, the socket's label, the e2e spec.
- [x] ADR 0014 §10: Windows as built.

## Acceptance criteria

- [x] On Windows, an engine's real fault (`DebugSegv`) produces a report whose frames name the engine module.
- [x] A client naming another process's ID gets no dump, and no process is opened (test).
- [x] The engine's sandbox selftest passes on Windows as the app starts the engine, which on Windows connects no handler (see As built); its network probe is blocked.

## Post-merge checks

None.

## Tests

The dump server's peer check (a unit or integration test on Windows CI), and D7d's integration and e2e tests extended to Windows.

## Docs to update

ADR 0014 §10, the desktop plan's §9 D7 to-do, `deny.toml` if a fork is pinned.

## Risks and open questions

- minidumper's maintainers may want the check done another way; a fork must be kept current until they take it.
- The engine's start-up impersonation (D2c) may change what it can connect to; test before and after the impersonation reverts.

## As built

These differ from the text above:

- **The engine takes no part on Windows.** The first version connected the engine's crash handler to the monitor's socket, as on Unix, with the socket labelled Low integrity so the sandboxed engine could reach it. CI's sandbox selftest then showed the engine opening a TCP connection: starting Winsock before lock-down left it working afterwards, and the Windows sandbox had counted on Winsock failing to start. So (owner's choice) the monitor watches the engines as their debugger instead (`laminapdf-desktop-crash`'s `capture/debugger.rs`): it finds the children of the main process that run the engine binary by scanning every 50 ms, attaches with kill-on-exit off, passes every exception on, and on a second-chance one writes the dump itself with `MiniDumpWriteDump`. No message names a process to it, so nothing can point it at another process; the engine opens no socket and gets no crash arguments.
- **What that leaves.** An engine that faults before the next scan has found it is reported without its `native` part. A fail-fast (`0xC0000409`, a panic's abort) leaves no dump: the panic's report is the crash's, and the engine's panic hook now asks for none on every OS (the app no longer waits for one after a panic).
- **The patched minidumper serves the main process.** The main process still connects to the monitor's main socket, so the peer check stays: `third_party/minidumper`, vendored in-tree with `[patch.crates-io]` (owner's choice, instead of a pinned git fork and a `deny.toml` source), to be offered upstream. A refused request drops only its client. The spec's "only a process the app started" isn't checked for that socket: a process that can reach it can have only itself dumped (ADR 0014 §10).
- **The monitor's messages reach the app's log**, without paths, so a monitor that attaches to nothing shows it.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
