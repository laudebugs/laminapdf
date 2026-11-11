# D2c — Sandbox the engine process on macOS, Windows and Linux

| Field | Value |
| --- | --- |
| ID | `D2c` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2b](../done/D2b-engine-process.md) |
| ADRs | Implements and amends ADR 0008 §sandboxing (records what is actually enforced per OS). Follows [Rust guidelines](../../guidelines/rust.md) (`unsafe` only with `// SAFETY:`), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/d2c-engine-sandbox` |
| PR title | `feat(desktop): sandbox the engine process per OS` |
| Size | M–L (400–900 lines, mostly per-OS code and tests) |
| PR | [#420](https://github.com/laudebugs/laminapdf/pull/420) |

## Goal

Reduce what a compromised engine process can do to (almost) nothing: no file system, no network, no spawning, no access to other processes, bounded memory and CPU. The engine only needs its stdin/stdout pipes, its already-loaded libraries (including PDFium) and memory. This is the plan's security requirement ("Child process with minimal privileges; OS sandboxing where available … scope decided in ADR 0008", [desktop plan §4 security](../../plans/desktop-app.md#security)) and its risk "Sandbox restrictions block mmap or IPC — prototype sandbox in D2 per OS; document what is enforced" ([§7](../../plans/desktop-app.md#7-risks)). Because D2b already keeps paths and file handles in the main process, the sandbox can be strict.

## Context

- ADR 0008 §sandboxing (intent, written in D2a) and D2b's protocol (the engine's only channel).
- Chromium's sandbox designs (primary sources to cite in ADR 0008): [Mac sandbox](https://chromium.googlesource.com/chromium/src/+/main/sandbox/mac/README.md) (Seatbelt profiles via `sandbox_init`-style APIs), [Windows sandbox](https://chromium.googlesource.com/chromium/src/+/main/docs/design/sandbox.md) (restricted tokens, job objects, alternate desktop, integrity levels), [Linux sandboxing](https://chromium.googlesource.com/chromium/src/+/main/sandbox/linux/README.md) (seccomp-bpf + namespaces).
- Apple: App Sandbox entitlements (`com.apple.security.app-sandbox`, `com.apple.security.inherit`) — a helper with its own `app-sandbox` entitlement runs sandboxed even inside a non-sandboxed app; hardened runtime is required for notarisation anyway (D6c).
- Windows: `CreateRestrictedToken`, `SetTokenInformation(TokenIntegrityLevel = Low/Untrusted)`, Job Objects (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS = 1`, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, UI restrictions), `PROCESS_MITIGATION_*` policies via `UpdateProcThreadAttribute` (no child processes, prohibit dynamic code if PDFium allows, image load restrictions), AppContainer (`CreateAppContainerProfile`) as the stronger option.
- Linux: Landlock (kernel ≥ 5.13; `landlock` crate, MIT/Apache) for fs/net denial, seccomp-bpf (`seccompiler` from Firecracker, Apache-2.0) with an allow-list of syscalls, `PR_SET_NO_NEW_PRIVS`, `setrlimit(RLIMIT_AS)`. **Not** `RLIMIT_NPROC`: on Linux it counts every process *and thread* of the real user ID (all of the user's processes, not just the engine), so `0` would make the engine's own `pthread_create` fail with `EAGAIN` and any non-zero value is meaningless across the user's session; process creation is denied with seccomp instead. User namespaces may be disabled by distro policy (Ubuntu 24.04 AppArmor restricts unprivileged userns) — do not depend on them.

## Scope

**In scope**

- `apps/desktop/src-tauri/src/engine/sandbox/{mod.rs,macos.rs,windows.rs,linux.rs}` (spawn-side policy) and `crates/laminapdf-desktop-engine/src/sandbox/` (self-restriction the engine applies at start-up where the OS allows it; the order differs per OS, see below):
  - **Lock-down order (per OS)** — fonts: PDFium's built-in base-14 fonts; system font access decided here (prefer passing font bytes from main on request over granting font directory reads). In every case no `OpenDocument` is read before the sandbox is fully on, and `Ready { sandboxed: SandboxLevel }` is sent last.
    - **Linux**: load PDFium → apply no-new-privs + Landlock + seccomp (all self-applied at run time) → `Ready`.
    - **macOS**: App Sandbox comes from the entitlement and is in force **from exec**, before `main` runs, so PDFium is loaded *inside* the sandbox. It must be readable there: ship it inside the bundle (`Contents/Frameworks` or `Contents/Resources`, which a sandboxed process can read) and verify `dlopen` works under the sandbox and library validation (same Team ID signature, D6c). If it doesn't, use the Seatbelt fallback below, which is applied at run time after loading (Linux-style order).
    - **Windows**: the Job Object and Low integrity level are set **at spawn** (a Low-IL process can still read the install directory, so PDFium loads); the stricter restricted token follows Chromium's pattern — the process is created with the locked-down primary token while its first thread impersonates a less restricted token for start-up, loads PDFium, then calls `RevertToSelf()` — plus process mitigation policies set after load (`SetProcessMitigationPolicy`: image-load policy blocking remote/low-label images, child-process policy). → `Ready`.
  - **macOS**: engine sidecar signed with an entitlements file `engine.entitlements` containing `com.apple.security.app-sandbox = true` (no file/network entitlements); verify the helper launches from a non-sandboxed parent and that stdio pipes work; fallback: Seatbelt profile via the private `sandbox_init_with_parameters` (Chromium does this) only if App Sandbox breaks pipes — record the choice.
  - **Windows**: spawn with a restricted token at Low integrity, in a Job Object with memory limit (configurable, default e.g. 2 GiB per engine), active-process limit 1, kill-on-close, UI restrictions; child-process creation disabled via `PROCESS_CREATION_CHILD_PROCESS_RESTRICTED`; evaluate AppContainer with no capabilities (record if pipes/PDFium work).
  - **Linux**: `prctl(PR_SET_NO_NEW_PRIVS)`, Landlock ruleset denying all fs access (best-effort ABI; report level achieved), seccomp-bpf allow-list (read/write on the inherited fds, `mmap`/`munmap`/`mprotect` without `PROT_EXEC` on new mappings if PDFium tolerates it, `futex`, `clock_gettime`, `exit_group`, `sched_yield`, `madvise`, `brk`, `rt_sigreturn`…; default action `SECCOMP_RET_KILL_PROCESS` in release, `SECCOMP_RET_TRAP`/log in a debug feature to discover missing syscalls), and `RLIMIT_AS`. **No new processes, threads still work**:
    - `clone` allowed only when its flags argument (arg 0 on x86_64 and aarch64) has `CLONE_THREAD` set (seccomp argument filter with `SECCOMP_CMP_MASKED_EQ`); without it → kill.
    - `clone3` → `SECCOMP_RET_ERRNO(ENOSYS)`: its flags sit behind a pointer seccomp cannot inspect, and glibc (≥ 2.34) and Rust's std fall back to `clone` on `ENOSYS`, which the rule above filters.
    - `fork`, `vfork` (x86_64 only; aarch64 has neither), `execve`, `execveat` → kill.
    - Landlock's deny-all ruleset also handles `LANDLOCK_ACCESS_FS_EXECUTE`, so even a missed exec path finds no executable file.
- `SandboxLevel` reported per engine to the main process and logged; a setting/env var `LAMINA_PDF_ENGINE_SANDBOX=off` only in debug builds.
- Supervisor treats an engine killed by the sandbox (SIGSYS, job memory limit) like a crash (restart policy from D2b) and logs the reason category.
- ADR 0008 amendment: a table "what is enforced per OS", known gaps, kernel/OS version requirements.

**Out of scope** (and which todo covers it)

- Signing the engine with entitlements in release builds: [D6c](../ready/D6c-macos-signing-and-notarisation.md) (this PR adds the entitlements file and ad-hoc signs in dev/CI).
- Crash/minidump capture of sandboxed engines: [D7a](../ready/D7a-crash-reporting.md).
- Sandboxing the webview processes (owned by the OS webview).
- Mac App Store sandboxing of the whole app (non-goal).

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] Add dependencies with licence comments: `landlock`, `seccompiler` (Linux, `cfg(target_os = "linux")`), `windows` crate features for security/job objects (`cfg(windows)`); no new deps on macOS if entitlements suffice.
- [x] Implement the per-OS start-up order above (Linux: all after load; macOS: from exec, PDFium loaded inside; Windows: Job + Low IL at spawn, impersonation token until PDFium is loaded, then `RevertToSelf` + mitigation policies).
- [x] Implement the per-OS modules behind a single `fn apply_engine_sandbox(cfg: &SandboxConfig) -> Result<SandboxLevel, SandboxError>` (engine side) and `fn spawn_sandboxed(cmd: Command, cfg: &SandboxConfig) -> Result<Child, SandboxError>` (main side); keep all `unsafe` FFI in these modules with `// SAFETY:` comments.
- [x] Font strategy: implement `FontRequest`/`FontData` protocol messages if PDFium needs system fonts for non-embedded fonts; main reads fonts from system directories; document in ADR 0008.
- [x] macOS: `apps/desktop/src-tauri/engine.entitlements`; ad-hoc sign the sidecar in the xtask `desktop-sidecar` step (`codesign -s - --entitlements … --options runtime`) so dev builds exercise the sandbox.
- [x] Probe binary/test mode: engine flag `--sandbox-selftest` that, after applying the sandbox, attempts forbidden actions (open `/etc/hosts` or `C:\Windows\win.ini`, TCP connect to 127.0.0.1, spawn a process, write to the user's home) and reports each as blocked/allowed, **and** attempts allowed actions that must keep working (spawn 4 `std::thread`s that each render a tile on the worker pool and join) — used by tests below. On Linux the forbidden probes run in a forked-before-sandbox helper or as separate selftest invocations, since a kill action ends the process (one probe per invocation: `--sandbox-selftest=<probe>`).
- [x] Measure and record: render time with and without sandbox (should be equal), engine startup time increase.
- [x] Amend ADR 0008 with the enforcement table.

## Acceptance criteria

- [x] On each OS, `--sandbox-selftest` reports **blocked** for: reading an arbitrary file, writing a file, opening a network socket, spawning a process. Any exception is listed in ADR 0008 with reason and OS version.
- [x] On each OS, the selftest's thread probe succeeds after the sandbox is applied (threads are created and join; tiles render on the worker pool); on Linux this holds with `clone3` returning `ENOSYS`.
- [x] All D2b integration tests still pass with the sandbox on (rendering, restart).
- [x] On Linux without Landlock (older kernel) the engine still starts with seccomp + no-new-privs and reports the reduced `SandboxLevel`; the app shows nothing to the user but logs it.
- [x] Windows engine memory limit enforced: a test document that forces excessive allocation (or a test-only "allocate N GiB" command under `e2e`) gets the engine killed and restarted, not the app.
- [x] No regression in the 1 GB open time from D2b (≤ 5 % slower).

## Post-merge checks

None.

## Tests

- `crates/laminapdf-desktop-engine/tests/sandbox_selftest.rs`: spawn the engine with `--sandbox-selftest` and assert each probe is blocked, per OS (`#[cfg(target_os = …)]`); runs in the three-OS desktop CI job.
- Supervisor test: engine killed by sandbox violation → restart path taken.
- Linux-only tests: seccomp default action kills on a forbidden syscall (selftest `socket()` probe → exit by `SIGSYS`); `fork()` and `execve("/bin/true")` probes are killed; a `clone` with `CLONE_THREAD` (via `std::thread::spawn`) succeeds; a raw `clone3` returns `ENOSYS`; `getrlimit(RLIMIT_NPROC)` is unchanged from the parent's value (guards against reintroducing the limit).

## Docs to update

- ADR 0008 (enforcement table, gaps, requirements).
- `docs/plans/desktop-app.md` §7 risk "Sandbox restrictions block mmap or IPC": add "see ADR 0008" result note; §9 D2 has no dedicated sandbox to-do — note completion under "Implement the engine child process…".
- `apps/desktop/README.md`: how to disable the sandbox in debug builds when diagnosing.

## Risks and open questions

- **PDFium needs `PROT_EXEC`/JIT?** It shouldn't (no V8 in our build — confirm the pinned PDFium build has V8/XFA disabled; PDF JavaScript must never run). If it does, the seccomp policy must allow it; record it.
- **macOS App Sandbox on a helper** may create a container directory and require a matching bundle identifier; test on macOS 13 and the newest release.
- **libc thread-creation paths**: musl or a future glibc might create threads through a syscall the filter doesn't expect; the thread probe catches it on CI; the debug `SECCOMP_RET_TRAP` feature names the syscall.
- **Ubuntu AppArmor userns restrictions** — we deliberately avoid namespaces; confirm the approach on Ubuntu 24.04 and Fedora.
- **Windows Low integrity + pipes**: inherited anonymous pipe handles work across integrity levels; named pipes need an explicit DACL. Use inherited handles only.
- **Font access** might push us toward granting read access to font directories; prefer the protocol approach.

## Results and deviations

- **One crate for both halves.** The spawn side and the engine side live in `crates/laminapdf-desktop-sandbox` (layer `app`, unpublished) instead of `apps/desktop/src-tauri/src/engine/sandbox/` and `crates/laminapdf-desktop-engine/src/sandbox/`, so the engine's tests start the real engine exactly as the app does (`SandboxSpawner`), on Windows too. The protocol crate's `CommandSpawner` moved there.
- **macOS: Seatbelt, not App Sandbox.** App Sandbox works for an ad-hoc signed engine once the binary embeds an `Info.plist` and PDFium sits in the bundle (and without `--options runtime`: library validation refuses PDFium without a Team ID), but it left `/etc/hosts` readable and `/usr/bin/true` startable, and a process in App Sandbox can't apply Seatbelt on top. The engine applies a deny-by-default Seatbelt profile with `sandbox_init` after loading PDFium; `engine.entitlements` is empty on purpose and `desktop-sidecar` signs ad hoc with it.
- **Windows:** the start-up token must be restricted itself (restricted to the user's own SIDs), or Windows drops the impersonation to identification level and the engine fails to load (STATUS_BAD_IMPERSONATION_LEVEL); the engine starts `DETACHED_PROCESS`, since attaching to a console needs the console driver, which the token can't open (STATUS_DLL_INIT_FAILED). AppContainer was evaluated on paper, not run: ADR 0008 §5 says why.
- **Fonts:** not implemented here. PDFium falls back to its built-in fonts, which keep Latin text's layout; CJK text in a non-embedded font loses its glyphs. [D2f](../ready/D2f-engine-system-fonts.md) adds `FontRequest`/`FontData`.
- **Memory limit test:** the selftest's `memory` probe allocates past the limit and is refused on Linux and Windows (macOS has no enforced limit, a recorded gap). The supervisor's restart after a sandbox kill is tested on Unix with a SIGSYS sent by the test, since nothing in a real engine can be made to violate the sandbox on purpose without test code in the shipped binary.
- **Measured:** ADR 0008 §5. Every open and tile figure is within 2 % of the same build with the sandbox off.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
