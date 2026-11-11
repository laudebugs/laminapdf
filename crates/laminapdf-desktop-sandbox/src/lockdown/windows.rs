//! Windows: the engine's half. The main process started it suspended, in a job, with a
//! locked-down Low-integrity primary token and its first thread impersonating a less restricted
//! one (`spawn/windows.rs`), so PDFium could load. Here, after loading:
//!
//! 1. `RevertToSelf` drops the impersonation; the thread runs on the primary token, which opens
//!    nothing (every group deny-only, the NULL SID its only restricting SID).
//! 2. The mitigation policies that would have stopped PDFium loading: only Microsoft-signed
//!    images from now on, and no dynamic code.
//! 3. A check of what the spawn set up (the job, Low integrity, the restricted token, the
//!    child-process policy), so an engine started some other way reports a reduced level.
#![expect(
    unsafe_code,
    reason = "FFI: RevertToSelf, SetProcessMitigationPolicy and token queries"
)]

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use laminapdf_desktop_protocol::SandboxLevel;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, IsTokenRestricted, RevertToSelf,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenIntegrityLevel,
};
use windows_sys::Win32::System::JobObjects::{
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, QueryInformationJobObject,
};
use windows_sys::Win32::System::SystemServices::SECURITY_MANDATORY_LOW_RID;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessMitigationPolicy, OpenProcessToken, PROCESS_MITIGATION_POLICY,
    ProcessChildProcessPolicy, ProcessDynamicCodePolicy, ProcessSignaturePolicy, SetProcessMitigationPolicy,
};

use crate::win32::{JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_PROCESS_MEMORY};
use crate::{SandboxConfig, SandboxError};

pub(super) fn apply(_config: SandboxConfig) -> Result<SandboxLevel, SandboxError> {
    // Opened while still impersonating: the locked-down token can't open even its own token.
    let token = Token::open()?;
    // SAFETY: RevertToSelf takes no arguments; without an impersonation token it does nothing.
    if unsafe { RevertToSelf() } == 0 {
        return Err(SandboxError::last_os("RevertToSelf"));
    }
    // Bit 0 of each policy's flags: MicrosoftSignedOnly, ProhibitDynamicCode.
    let signed = set_policy(ProcessSignaturePolicy, 1);
    let no_dynamic_code = set_policy(ProcessDynamicCodePolicy, 1);
    if !signed || !no_dynamic_code {
        super::log("a post-load mitigation policy was refused");
    }
    // NoChildProcessCreation, set at creation (PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY).
    let no_children = policy(ProcessChildProcessPolicy).is_some_and(|flags| flags & 1 == 1);
    let (job, memory_limit) = our_job();
    Ok(SandboxLevel {
        job,
        memory_limit,
        low_integrity: token
            .integrity()
            .is_some_and(|rid| rid <= SECURITY_MANDATORY_LOW_RID as u32),
        restricted_token: token.restricted(),
        mitigations: no_children && signed && no_dynamic_code,
        ..SandboxLevel::default()
    })
}

/// Sets one process mitigation policy whose structure is a single `u32` of flags.
fn set_policy(policy: PROCESS_MITIGATION_POLICY, flags: u32) -> bool {
    // SAFETY: both policies' structures (PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY,
    // PROCESS_MITIGATION_DYNAMIC_CODE_POLICY) are one u32 of flags; the pointer and the length
    // describe `flags`, which outlives the call.
    unsafe { SetProcessMitigationPolicy(policy, (&raw const flags).cast::<c_void>(), size_of::<u32>()) != 0 }
}

/// Reads a mitigation policy whose structure is a single `u32` of flags.
fn policy(policy: PROCESS_MITIGATION_POLICY) -> Option<u32> {
    let mut flags = 0u32;
    // SAFETY: the current-process pseudo-handle needs no closing; `flags` is a writable u32 of
    // the length given, the size of PROCESS_MITIGATION_CHILD_PROCESS_POLICY.
    let ok = unsafe {
        GetProcessMitigationPolicy(
            GetCurrentProcess(),
            policy,
            (&raw mut flags).cast::<c_void>(),
            size_of::<u32>(),
        )
    };
    (ok != 0).then_some(flags)
}

/// Whether the engine's immediate job is the one the main process made (one active process,
/// a process memory limit), and whether that limit is set. Not just any job: CI runners and
/// some launchers put every process in one of their own.
fn our_job() -> (bool, bool) {
    // SAFETY: a zeroed JOBOBJECT_EXTENDED_LIMIT_INFORMATION is valid; a null job handle queries
    // the calling process's immediate job; the pointer and length describe `info`.
    let info = unsafe {
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
        let ok = QueryInformationJobObject(
            null_mut(),
            JobObjectExtendedLimitInformation,
            (&raw mut info).cast::<c_void>(),
            u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()).unwrap_or(u32::MAX),
            null_mut(),
        );
        if ok == 0 {
            return (false, false);
        }
        info
    };
    let flags = info.BasicLimitInformation.LimitFlags;
    let one_process =
        flags & JOB_OBJECT_LIMIT_ACTIVE_PROCESS != 0 && info.BasicLimitInformation.ActiveProcessLimit == 1;
    let memory = flags & JOB_OBJECT_LIMIT_PROCESS_MEMORY != 0 && info.ProcessMemoryLimit > 0;
    (one_process, one_process && memory)
}

/// The process's primary token, opened for queries.
struct Token(HANDLE);

impl Token {
    fn open() -> Result<Self, SandboxError> {
        let mut handle: HANDLE = null_mut();
        // SAFETY: `handle` is a writable HANDLE that receives the token, closed in `drop`.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut handle) } == 0 {
            return Err(SandboxError::last_os("OpenProcessToken"));
        }
        Ok(Self(handle))
    }

    fn restricted(&self) -> bool {
        // SAFETY: `self.0` is an open token handle.
        unsafe { IsTokenRestricted(self.0) != 0 }
    }

    /// The integrity level's RID (`SECURITY_MANDATORY_LOW_RID` is 0x1000).
    fn integrity(&self) -> Option<u32> {
        let mut buffer = [0u64; 16];
        let mut needed = 0u32;
        // SAFETY: `buffer` is 128 writable bytes, 8-byte aligned, enough for a
        // TOKEN_MANDATORY_LABEL and its SID; `needed` receives the length used.
        let ok = unsafe {
            GetTokenInformation(
                self.0,
                TokenIntegrityLevel,
                buffer.as_mut_ptr().cast::<c_void>(),
                u32::try_from(size_of::<[u64; 16]>()).unwrap_or(u32::MAX),
                &raw mut needed,
            )
        };
        if ok == 0 {
            return None;
        }
        // SAFETY: on success the buffer starts with a TOKEN_MANDATORY_LABEL whose SID pointer
        // points into the buffer; GetSidSubAuthorityCount/GetSidSubAuthority read that valid SID,
        // and the last sub-authority of an integrity SID is its level.
        unsafe {
            let label = buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>().read();
            let sid = label.Label.Sid;
            let count = *GetSidSubAuthorityCount(sid);
            Some(*GetSidSubAuthority(sid, u32::from(count.checked_sub(1)?)))
        }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the token this value opened, closed once.
        unsafe { CloseHandle(self.0) };
    }
}
