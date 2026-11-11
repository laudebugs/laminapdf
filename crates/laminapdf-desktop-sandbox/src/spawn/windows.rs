//! Windows: the engine starts inside its sandbox, following Chromium's broker/target pattern
//! (<https://chromium.googlesource.com/chromium/src/+/main/docs/design/sandbox.md>):
//!
//! - **The primary token is locked down** from the start: a restricted copy of the user's token
//!   with every group (the user's SID too) deny-only, every privilege but `SeChangeNotify` gone,
//!   the NULL SID as its only restricting SID (so no access check can succeed), at Low integrity.
//! - **The first thread impersonates** a less restricted token (the user's groups, also as its
//!   restricting SIDs, so it has the user's access but counts as restricted; no privileges; Low
//!   integrity) so the loader and PDFium can read the install folder; the engine drops it
//!   with `RevertToSelf` once PDFium is loaded (`lockdown/windows.rs`).
//! - **A job object** holds it from creation (`PROC_THREAD_ATTRIBUTE_JOB_LIST`): a committed-
//!   memory limit, one active process, killed when the main process's handle closes, and the
//!   UI restrictions (no other desktop, clipboard, global atoms or other processes' windows).
//! - **Mitigation policies** set at creation: no child processes, no images from remote shares
//!   or Low-labelled files, no legacy extension points, no non-system fonts, strict handles.
//! - **Only its three pipe ends are inherited** (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`): anonymous
//!   pipes, so no named-pipe DACL is involved and the integrity level doesn't matter to them.
#![expect(
    unsafe_code,
    reason = "FFI: tokens, job objects, process attributes, CreateProcessAsUserW"
)]

use std::ffi::{OsStr, c_void};
use std::io;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt as _;
use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
use std::process::Command;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HANDLE, HANDLE_FLAG_INHERIT, SetHandleInformation};
use windows_sys::Win32::Security::{
    AllocateAndInitializeSid, CreateRestrictedToken, DuplicateTokenEx, FreeSid, GetTokenInformation, PSID,
    SECURITY_ATTRIBUTES, SID_AND_ATTRIBUTES, SID_IDENTIFIER_AUTHORITY, SecurityImpersonation, SetTokenInformation,
    TOKEN_GROUPS, TOKEN_MANDATORY_LABEL, TOKEN_USER, TokenGroups, TokenImpersonation, TokenIntegrityLevel, TokenUser,
};
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicUIRestrictions, JobObjectExtendedLimitInformation, QueryInformationJobObject,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessAsUserW, DeleteProcThreadAttributeList, GetCurrentProcess, GetExitCodeProcess, INFINITE,
    InitializeProcThreadAttributeList, OpenProcessToken, PROCESS_INFORMATION, ResumeThread, STARTUPINFOEXW,
    SetThreadToken, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::{Inner, SandboxedChild, SandboxedProcess};
use crate::win32::{JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_PROCESS_MEMORY};
use crate::{SandboxConfig, SandboxError};

// winnt.h / winbase.h values, spelled out so each is next to its meaning.
const TOKEN_ASSIGN_PRIMARY: u32 = 0x0001;
const TOKEN_DUPLICATE: u32 = 0x0002;
const TOKEN_IMPERSONATE: u32 = 0x0004;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ADJUST_DEFAULT: u32 = 0x0080;
const DISABLE_MAX_PRIVILEGE: u32 = 0x1;
const SE_GROUP_INTEGRITY: u32 = 0x20;
const SECURITY_MANDATORY_LOW_RID: u32 = 0x1000;
const STARTF_USESTDHANDLES: u32 = 0x100;
const CREATE_SUSPENDED: u32 = 0x4;
const CREATE_UNICODE_ENVIRONMENT: u32 = 0x400;
const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
const DETACHED_PROCESS: u32 = 0x8;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x0002_0002;
const PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY: usize = 0x0002_0007;
const PROC_THREAD_ATTRIBUTE_JOB_LIST: usize = 0x0002_000D;
const PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY: usize = 0x0002_000E;
const PROCESS_CREATION_CHILD_PROCESS_RESTRICTED: u32 = 0x1;
const JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION: u32 = 0x400;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
/// Every `JOB_OBJECT_UILIMIT_*` flag: handles, clipboard, system parameters, display settings,
/// global atoms, desktop, exiting Windows.
const JOB_OBJECT_UILIMIT_ALL: u32 = 0xFF;
/// `PROCESS_CREATION_MITIGATION_POLICY_*_ALWAYS_ON`, each 1 at its position in the 64-bit mask.
const MITIGATIONS: u64 = (1 << 12) // heap: terminate on corruption
    | (1 << 16) // bottom-up ASLR
    | (1 << 20) // high-entropy ASLR
    | (1 << 24) // strict handle checks
    | (1 << 32) // no legacy extension points (AppInit DLLs, hooks)
    | (1 << 48) // no non-system fonts
    | (1 << 52) // no images from remote devices
    | (1 << 56) // no images labelled Low integrity
    | (1 << 60); // prefer System32 images
/// `STATUS_NO_MEMORY` and `STATUS_STACK_BUFFER_OVERRUN` (Rust's abort after a failed allocation).
const OUT_OF_MEMORY_CODES: [u32; 2] = [0xC000_0017, 0xC000_0409];

/// The engine's process and the job that holds it; dropping the job's handle kills it.
pub(super) struct JobProcess {
    process: OwnedHandle,
    job: OwnedHandle,
    pid: u32,
    memory_limit: u64,
}

impl JobProcess {
    pub(super) const fn id(&self) -> u32 {
        self.pid
    }

    pub(super) fn kill(&mut self) {
        // SAFETY: `self.job` is an open job handle; terminating an empty job does nothing.
        unsafe { TerminateJobObject(self.job.as_raw_handle(), 1) };
    }

    pub(super) fn wait(&mut self) -> String {
        let process = self.process.as_raw_handle();
        let mut code = 0u32;
        // SAFETY: `process` is an open process handle with SYNCHRONIZE and
        // PROCESS_QUERY_INFORMATION (CreateProcess gives full access); `code` is writable.
        let ok =
            unsafe { WaitForSingleObject(process, INFINITE) == 0 && GetExitCodeProcess(process, &raw mut code) != 0 };
        if !ok {
            return format!("unknown ({})", io::Error::last_os_error());
        }
        let peak = self.peak_memory();
        if OUT_OF_MEMORY_CODES.contains(&code) && peak >= self.memory_limit / 10 * 9 {
            return format!(
                "memory limit: the job stopped it at {} MiB (limit {} MiB, exit code {code:#010x})",
                peak >> 20,
                self.memory_limit >> 20
            );
        }
        if code >= 0xC000_0000 {
            return format!("crash: exit code {code:#010x}");
        }
        format!("exit code: {code}")
    }

    /// The most committed memory the engine ever had, from the job's accounting.
    fn peak_memory(&self) -> u64 {
        // SAFETY: a zeroed JOBOBJECT_EXTENDED_LIMIT_INFORMATION is valid; the pointer and length
        // describe it, and `self.job` is an open job handle with JOB_OBJECT_QUERY.
        unsafe {
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            let ok = QueryInformationJobObject(
                self.job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw mut info).cast::<c_void>(),
                size32::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>(),
                null_mut(),
            );
            if ok == 0 { 0 } else { info.PeakProcessMemoryUsed as u64 }
        }
    }
}

pub(super) fn spawn(command: &Command, config: &SandboxConfig) -> Result<SandboxedChild, SandboxError> {
    let (stdin_child, stdin) = io::pipe()?;
    let (stdout, stdout_child) = io::pipe()?;
    let (stderr, stderr_child) = io::pipe()?;
    let inherited = [
        stdin_child.as_raw_handle(),
        stdout_child.as_raw_handle(),
        stderr_child.as_raw_handle(),
    ];
    for handle in inherited {
        // SAFETY: `handle` is an open pipe handle this function owns.
        if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) } == 0 {
            return Err(SandboxError::last_os("SetHandleInformation"));
        }
    }
    let job = job(config.memory_limit)?;
    let tokens = Tokens::new()?;
    let (process, thread, pid) = create(command, &tokens, &job, &inherited)?;
    let started = JobProcess {
        process,
        job,
        pid,
        memory_limit: config.memory_limit,
    };
    let thread_handle = thread.as_raw_handle();
    // SAFETY: `thread_handle` is the suspended main thread (THREAD_SET_THREAD_TOKEN: full access
    // from CreateProcess); the impersonation token is open with TOKEN_IMPERSONATE.
    if unsafe { SetThreadToken(&raw const thread_handle, tokens.impersonation.as_raw_handle()) } == 0 {
        let error = SandboxError::last_os("SetThreadToken");
        // SAFETY: the process handle is open; the suspended process is ended before it ran.
        unsafe { TerminateProcess(started.process.as_raw_handle(), 1) };
        return Err(error);
    }
    // SAFETY: `thread_handle` is the suspended main thread; resuming it once starts the engine.
    if unsafe { ResumeThread(thread_handle) } == u32::MAX {
        let error = SandboxError::last_os("ResumeThread");
        // SAFETY: as above.
        unsafe { TerminateProcess(started.process.as_raw_handle(), 1) };
        return Err(error);
    }
    // The child's ends close here; the engine holds its own copies.
    drop((stdin_child, stdout_child, stderr_child));
    Ok(SandboxedChild {
        stdin: Box::new(stdin),
        stdout: Box::new(stdout),
        stderr: Box::new(stderr),
        process: SandboxedProcess(Inner::Job(started)),
    })
}

/// The job: committed-memory limit, one process, killed with the last handle, UI restrictions.
fn job(memory_limit: u64) -> Result<OwnedHandle, SandboxError> {
    // SAFETY: null attributes and name create an unnamed job with the default security; the
    // handle returned is owned here.
    let job = unsafe { CreateJobObjectW(null(), null()) };
    if job.is_null() {
        return Err(SandboxError::last_os("CreateJobObjectW"));
    }
    // SAFETY: `job` is the new, open job handle, owned from here on.
    let job = unsafe { OwnedHandle::from_raw_handle(job) };
    // SAFETY: zeroed limit structures are valid; each pointer and length describe a structure
    // that outlives its call, and `job` is open with JOB_OBJECT_SET_ATTRIBUTES.
    unsafe {
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.ProcessMemoryLimit = usize::try_from(memory_limit).unwrap_or(usize::MAX);
        if SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast::<c_void>(),
            size32::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>(),
        ) == 0
        {
            return Err(SandboxError::last_os("the job's limits"));
        }
        let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
            UIRestrictionsClass: JOB_OBJECT_UILIMIT_ALL,
        };
        if SetInformationJobObject(
            job.as_raw_handle(),
            JobObjectBasicUIRestrictions,
            (&raw const ui).cast::<c_void>(),
            size32::<JOBOBJECT_BASIC_UI_RESTRICTIONS>(),
        ) == 0
        {
            return Err(SandboxError::last_os("the job's UI restrictions"));
        }
    }
    Ok(job)
}

/// The engine's two tokens: the locked-down primary and the start-up impersonation token.
struct Tokens {
    primary: OwnedHandle,
    impersonation: OwnedHandle,
}

impl Tokens {
    fn new() -> Result<Self, SandboxError> {
        let mut raw: HANDLE = null_mut();
        let access = TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT;
        // SAFETY: the current-process pseudo-handle needs no closing; `raw` receives the token.
        if unsafe { OpenProcessToken(GetCurrentProcess(), access, &raw mut raw) } == 0 {
            return Err(SandboxError::last_os("OpenProcessToken"));
        }
        // SAFETY: `raw` is the open token, owned from here on.
        let own = unsafe { OwnedHandle::from_raw_handle(raw) };
        let user = query(&own, TokenUser)?;
        let groups = query(&own, TokenGroups)?;
        // SAFETY: `user` holds a TOKEN_USER and `groups` a TOKEN_GROUPS as GetTokenInformation
        // wrote them, 8-byte aligned; the SID pointers inside point into the same buffers, which
        // live until the tokens below are made.
        // Every SID of the user's token but its integrity label.
        let sids: Vec<SID_AND_ATTRIBUTES> = unsafe {
            let user = user.as_ptr().cast::<TOKEN_USER>().read();
            let groups = groups.as_ptr().cast::<TOKEN_GROUPS>();
            let count = (*groups).GroupCount as usize;
            let first = (&raw const (*groups).Groups).cast::<SID_AND_ATTRIBUTES>();
            std::iter::once(user.User)
                .chain((0..count).map(|i| first.add(i).read()))
                .filter(|sid| sid.Attributes & SE_GROUP_INTEGRITY == 0)
                .map(|sid| SID_AND_ATTRIBUTES {
                    Sid: sid.Sid,
                    Attributes: 0,
                })
                .collect()
        };
        let null_sid = Sid::null()?;
        let restrict = [SID_AND_ATTRIBUTES {
            Sid: null_sid.0,
            Attributes: 0,
        }];
        let primary = restricted(&own, &sids, &restrict)?;
        set_low_integrity(&primary)?;
        // Restricted to the user's own SIDs: the same access, but a restricted token, which is what a
        // thread of a process with a restricted primary token may impersonate (otherwise Windows
        // drops it to identification level and the loader fails, STATUS_BAD_IMPERSONATION_LEVEL).
        let start_up = restricted(&own, &[], &sids)?;
        set_low_integrity(&start_up)?;
        let mut raw: HANDLE = null_mut();
        // SAFETY: `start_up` is an open primary token with TOKEN_DUPLICATE; `raw` receives the
        // impersonation copy.
        let ok = unsafe {
            DuplicateTokenEx(
                start_up.as_raw_handle(),
                TOKEN_IMPERSONATE | TOKEN_QUERY,
                null(),
                SecurityImpersonation,
                TokenImpersonation,
                &raw mut raw,
            )
        };
        if ok == 0 {
            return Err(SandboxError::last_os("DuplicateTokenEx"));
        }
        // SAFETY: `raw` is the new impersonation token, owned from here on.
        let impersonation = unsafe { OwnedHandle::from_raw_handle(raw) };
        Ok(Self { primary, impersonation })
    }
}

/// `CreateRestrictedToken` with every privilege but `SeChangeNotify` removed.
fn restricted(
    token: &OwnedHandle,
    disable: &[SID_AND_ATTRIBUTES],
    restrict: &[SID_AND_ATTRIBUTES],
) -> Result<OwnedHandle, SandboxError> {
    let mut raw: HANDLE = null_mut();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let ptr = |s: &[SID_AND_ATTRIBUTES]| if s.is_empty() { null() } else { s.as_ptr() };
    // SAFETY: `token` is open with TOKEN_DUPLICATE | TOKEN_QUERY; each array pointer is null with
    // a count of 0 or points to `count` valid entries whose SIDs outlive the call; `raw` receives
    // the new token.
    let ok = unsafe {
        CreateRestrictedToken(
            token.as_raw_handle(),
            DISABLE_MAX_PRIVILEGE,
            count(disable.len()),
            ptr(disable),
            0,
            null(),
            count(restrict.len()),
            ptr(restrict),
            &raw mut raw,
        )
    };
    if ok == 0 {
        return Err(SandboxError::last_os("CreateRestrictedToken"));
    }
    // SAFETY: `raw` is the new token, owned from here on.
    Ok(unsafe { OwnedHandle::from_raw_handle(raw) })
}

/// Lowers `token` to Low integrity.
fn set_low_integrity(token: &OwnedHandle) -> Result<(), SandboxError> {
    let low = Sid::new([0, 0, 0, 0, 0, 16], SECURITY_MANDATORY_LOW_RID)?;
    let label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: low.0,
            Attributes: SE_GROUP_INTEGRITY,
        },
    };
    // SAFETY: `token` is open with TOKEN_ADJUST_DEFAULT (CreateRestrictedToken's handle has full
    // access); `label` and its SID outlive the call.
    let ok = unsafe {
        SetTokenInformation(
            token.as_raw_handle(),
            TokenIntegrityLevel,
            (&raw const label).cast::<c_void>(),
            size32::<TOKEN_MANDATORY_LABEL>(),
        )
    };
    if ok == 0 {
        return Err(SandboxError::last_os("SetTokenInformation(TokenIntegrityLevel)"));
    }
    Ok(())
}

/// A SID from `AllocateAndInitializeSid`, freed on drop.
struct Sid(PSID);

impl Sid {
    /// One sub-authority under `authority`.
    fn new(authority: [u8; 6], rid: u32) -> Result<Self, SandboxError> {
        let authority = SID_IDENTIFIER_AUTHORITY { Value: authority };
        let mut sid: PSID = null_mut();
        // SAFETY: `authority` outlives the call; one sub-authority is used and the rest are 0;
        // `sid` receives a SID freed with FreeSid in `drop`.
        let ok = unsafe { AllocateAndInitializeSid(&raw const authority, 1, rid, 0, 0, 0, 0, 0, 0, 0, &raw mut sid) };
        if ok == 0 {
            return Err(SandboxError::last_os("AllocateAndInitializeSid"));
        }
        Ok(Self(sid))
    }

    /// S-1-0-0, the NULL SID: no object grants it anything.
    fn null() -> Result<Self, SandboxError> {
        Self::new([0; 6], 0)
    }
}

impl Drop for Sid {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from AllocateAndInitializeSid and is freed once.
        unsafe { FreeSid(self.0) };
    }
}

/// A token's information of `class`, in an 8-byte-aligned buffer.
fn query(token: &OwnedHandle, class: i32) -> Result<Vec<u64>, SandboxError> {
    let mut needed = 0u32;
    // SAFETY: a null buffer of length 0 asks only for the length, written to `needed`.
    unsafe { GetTokenInformation(token.as_raw_handle(), class, null_mut(), 0, &raw mut needed) };
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: `buffer` holds at least `needed` writable bytes, 8-byte aligned.
    let ok = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            class,
            buffer.as_mut_ptr().cast::<c_void>(),
            needed,
            &raw mut needed,
        )
    };
    if ok == 0 {
        return Err(SandboxError::last_os("GetTokenInformation"));
    }
    Ok(buffer)
}

/// Creates the suspended engine: returns its process, its main thread and its id.
fn create(
    command: &Command,
    tokens: &Tokens,
    job: &OwnedHandle,
    inherited: &[HANDLE; 3],
) -> Result<(OwnedHandle, OwnedHandle, u32), SandboxError> {
    let program = wide(&executable(command.get_program()));
    let mut line = command_line(command);
    let mitigations = MITIGATIONS;
    let child_policy = PROCESS_CREATION_CHILD_PROCESS_RESTRICTED;
    let jobs = [job.as_raw_handle()];
    let mut size = 0usize;
    // SAFETY: a null list asks for the size of a list of four attributes, written to `size`.
    unsafe { InitializeProcThreadAttributeList(null_mut(), 4, 0, &raw mut size) };
    let mut list = vec![0u64; size.div_ceil(8)];
    let list_ptr = list.as_mut_ptr().cast::<c_void>();
    // SAFETY: `list` holds `size` writable bytes, 8-byte aligned.
    if unsafe { InitializeProcThreadAttributeList(list_ptr, 4, 0, &raw mut size) } == 0 {
        return Err(SandboxError::last_os("InitializeProcThreadAttributeList"));
    }
    let attributes: [(usize, *const c_void, usize); 4] = [
        (
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            inherited.as_ptr().cast(),
            size_of::<[HANDLE; 3]>(),
        ),
        (
            PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY,
            (&raw const mitigations).cast(),
            size_of::<u64>(),
        ),
        (
            PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY,
            (&raw const child_policy).cast(),
            size_of::<u32>(),
        ),
        (
            PROC_THREAD_ATTRIBUTE_JOB_LIST,
            jobs.as_ptr().cast(),
            size_of::<[HANDLE; 1]>(),
        ),
    ];
    let result = (|| {
        for (attribute, value, len) in attributes {
            // SAFETY: `list_ptr` is an initialised list; `value` points to `len` bytes that live
            // until CreateProcessAsUserW below has returned (they are this function's locals).
            if unsafe { UpdateProcThreadAttribute(list_ptr, 0, attribute, value, len, null_mut(), null()) } == 0 {
                return Err(SandboxError::last_os("UpdateProcThreadAttribute"));
            }
        }
        // SAFETY: a zeroed STARTUPINFOEXW and PROCESS_INFORMATION are valid.
        let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
        startup.StartupInfo.cb = size32::<STARTUPINFOEXW>();
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = inherited[0];
        startup.StartupInfo.hStdOutput = inherited[1];
        startup.StartupInfo.hStdError = inherited[2];
        startup.lpAttributeList = list_ptr;
        // SAFETY: as above.
        let mut info: PROCESS_INFORMATION = unsafe { zeroed() };
        // No console at all (not even a hidden one): attaching to a console means opening the console
        // driver, which the locked-down token can't, and the process would fail to start
        // (STATUS_DLL_INIT_FAILED). Its stdio is the three pipes.
        let flags = CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | DETACHED_PROCESS | CREATE_UNICODE_ENVIRONMENT;
        // SAFETY: the token is a primary token restricted from this process's own (so no
        // SeAssignPrimaryTokenPrivilege is needed); `program` and `line` are NUL-terminated wide
        // strings, `line` writable; null attributes, environment and directory inherit this
        // process's; `startup` is a STARTUPINFOEXW with an initialised attribute list whose values
        // outlive the call; `info` receives two handles owned below.
        let ok = unsafe {
            CreateProcessAsUserW(
                tokens.primary.as_raw_handle(),
                program.as_ptr(),
                line.as_mut_ptr(),
                null::<SECURITY_ATTRIBUTES>(),
                null::<SECURITY_ATTRIBUTES>(),
                1,
                flags,
                null(),
                null(),
                &raw const startup.StartupInfo,
                &raw mut info,
            )
        };
        if ok == 0 {
            return Err(SandboxError::last_os("CreateProcessAsUserW"));
        }
        // SAFETY: CreateProcessAsUserW returned two open handles, each owned once from here.
        Ok(unsafe {
            (
                OwnedHandle::from_raw_handle(info.hProcess),
                OwnedHandle::from_raw_handle(info.hThread),
                info.dwProcessId,
            )
        })
    })();
    // SAFETY: `list_ptr` was initialised above and is deleted once, after its last use.
    unsafe { DeleteProcThreadAttributeList(list_ptr) };
    result
}

/// `size_of::<T>()` as the `u32` Windows' length arguments take (every structure here is small).
fn size32<T>() -> u32 {
    u32::try_from(size_of::<T>()).unwrap_or(u32::MAX)
}

/// `program`, with `.exe` added when it names no extension and that file exists, as `Command`
/// does: `CreateProcessAsUserW` takes the file name as it is.
fn executable(program: &OsStr) -> std::ffi::OsString {
    let path = std::path::Path::new(program);
    let exe = path.with_extension("exe");
    if path.extension().is_none() && exe.is_file() {
        exe.into_os_string()
    } else {
        program.to_os_string()
    }
}

/// `s` as a NUL-terminated wide string.
fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// The program and its arguments as one command line, quoted the way the C runtime and Rust's
/// std parse it back (`CommandLineToArgvW`'s rules), NUL-terminated.
fn command_line(command: &Command) -> Vec<u16> {
    let mut line = Vec::new();
    push_quoted(&mut line, command.get_program());
    for arg in command.get_args() {
        line.push(u16::from(b' '));
        push_quoted(&mut line, arg);
    }
    line.push(0);
    line
}

fn push_quoted(line: &mut Vec<u16>, arg: &OsStr) {
    let chars: Vec<u16> = arg.encode_wide().collect();
    let plain = !chars.is_empty()
        && !chars
            .iter()
            .any(|&c| c == u16::from(b' ') || c == u16::from(b'\t') || c == u16::from(b'"'));
    if plain {
        line.extend(chars);
        return;
    }
    let backslash = u16::from(b'\\');
    line.push(u16::from(b'"'));
    let mut run = 0;
    for c in chars {
        if c == backslash {
            run += 1;
            continue;
        }
        // Backslashes before a quote are doubled and the quote escaped; others stay as they are.
        let times = if c == u16::from(b'"') { run * 2 + 1 } else { run };
        line.extend(std::iter::repeat_n(backslash, times));
        line.push(c);
        run = 0;
    }
    // Backslashes before the closing quote are doubled.
    line.extend(std::iter::repeat_n(backslash, run * 2));
    line.push(u16::from(b'"'));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quoted(arg: &str) -> String {
        let mut line = Vec::new();
        push_quoted(&mut line, OsStr::new(arg));
        String::from_utf16(&line).unwrap_or_default()
    }

    #[test]
    fn a_plain_argument_should_stay_unquoted() {
        assert_eq!(quoted(r"C:\a\b.exe"), r"C:\a\b.exe");
    }

    #[test]
    fn spaces_quotes_and_trailing_backslashes_should_round_trip() {
        assert_eq!(quoted(r#"a b\"c\"#), r#""a b\\\"c\\""#);
    }

    #[test]
    fn an_empty_argument_should_be_a_pair_of_quotes() {
        assert_eq!(quoted(""), r#""""#);
    }
}
