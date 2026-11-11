//! The winnt.h values both halves of the Windows sandbox use.

/// The job limits its number of active processes (`ActiveProcessLimit`).
pub(crate) const JOB_OBJECT_LIMIT_ACTIVE_PROCESS: u32 = 0x8;
/// The job limits each process's committed memory (`ProcessMemoryLimit`).
pub(crate) const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x100;
