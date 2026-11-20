//! macOS: a process's physical footprint (`proc_pid_rusage`'s `ri_phys_footprint`, what Activity
//! Monitor shows as Memory). The main process reads the engine's to enforce its memory limit; in
//! App Sandbox, where a process may read only its own, the engine reads its own (ADR 0014 §11).
#![expect(unsafe_code, reason = "FFI: proc_pid_rusage")]

use std::io;

/// The process's physical footprint in bytes.
pub(crate) fn footprint(pid: u32) -> io::Result<u64> {
    let pid = libc::c_int::try_from(pid).map_err(|_| io::Error::other("a pid past c_int"))?;
    // SAFETY: a zeroed rusage_info_v2 is valid plain data; proc_pid_rusage writes at most one
    // rusage_info_v2 (the flavour asked for) through the pointer, which is valid for it.
    unsafe {
        let mut info: libc::rusage_info_v2 = std::mem::zeroed();
        let status = libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, (&raw mut info).cast());
        if status == 0 {
            Ok(info.ri_phys_footprint)
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
