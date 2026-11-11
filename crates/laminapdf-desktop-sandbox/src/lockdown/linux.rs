//! Linux: everything is applied at run time, after PDFium is loaded, in this order:
//!
//! 1. `PR_SET_NO_NEW_PRIVS`: no set-uid or file-capability gain through `execve`, and what lets
//!    an unprivileged process install the next two.
//! 2. Landlock (kernel ≥ 5.13): a ruleset that handles every file-system right, TCP bind and
//!    connect and the IPC scopes the kernel knows, and allows none of them. Best effort: an older
//!    kernel enforces less or nothing, and [`SandboxLevel`] says what it got.
//! 3. `RLIMIT_AS`: twice [`SandboxConfig::memory_limit`]; never `RLIMIT_NPROC`, which counts every
//!    process and thread of the user, not the engine's.
//! 4. seccomp-bpf, three filters (the strictest answer of all installed filters wins):
//!    - an allow-list whose default kills the process: memory without `PROT_EXEC`, futexes,
//!      clocks, signals to itself, reads and writes on open descriptors, `clone` only with
//!      `CLONE_THREAD`. Not `socket`, `fork`, `vfork`, `execve` or `execveat`;
//!    - the calls that name a path (`openat`, `newfstatat`, …) fail with `EACCES`, so PDFium's
//!      font lookup finds nothing instead of killing the engine, Landlock or not; so does
//!      `sysinfo`;
//!    - `clone3` fails with `ENOSYS`: its flags sit behind a pointer the filter can't read, and
//!      glibc (≥ 2.34) and Rust's std then fall back to `clone`, which the first filter checks.
//!
//! Chromium's Linux sandbox is the model (seccomp-bpf with `EPERM` brokers for paths); it adds
//! namespaces, which distributions may forbid (Ubuntu 24.04 restricts unprivileged user
//! namespaces), so this sandbox does without them.
#![expect(
    unsafe_code,
    reason = "FFI: prctl, setrlimit, the Landlock version query, the SIGSYS handler"
)]

use std::collections::BTreeMap;

use laminapdf_desktop_protocol::{Layer, SandboxLevel};
use landlock::{
    ABI, Access, AccessFs, AccessNet, LandlockStatus, Ruleset, RulesetAttr, RulesetCreated, RulesetStatus, Scope,
};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter, SeccompRule, TargetArch,
};

use crate::{SandboxConfig, SandboxError};

/// `prctl(PR_SET_VMA, PR_SET_VMA_ANON_NAME, …)` (Linux 5.17), which the libc crate lacks.
const PR_SET_VMA: u64 = 0x5356_4d41;

/// The newest Landlock ABI the `landlock` crate knows; the kernel's own may be older.
const LANDLOCK_ABI: ABI = ABI::V9;

pub(super) fn apply(config: SandboxConfig) -> Result<SandboxLevel, SandboxError> {
    // Landlock and `RLIMIT_AS` hold for the whole process only if no other thread exists yet.
    // Unreadable /proc means neither can be checked: fail closed.
    let threads = std::fs::read_dir("/proc/self/task")
        .map_err(|source| SandboxError::Os {
            step: "listing /proc/self/task",
            source,
        })?
        .count();
    if threads > 1 {
        return Err(SandboxError::Threads(threads));
    }
    let inherited = inherited_descriptors()?;
    if inherited > 0 {
        return Err(SandboxError::Inherited(inherited));
    }
    // SAFETY: prctl with PR_SET_NO_NEW_PRIVS takes four further integer arguments, here 1, 0, 0, 0;
    // it reads no memory.
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(SandboxError::last_os("PR_SET_NO_NEW_PRIVS"));
    }
    let landlock = landlock();
    let limit = libc::rlimit {
        rlim_cur: config.memory_limit.saturating_mul(2),
        rlim_max: config.memory_limit.saturating_mul(2),
    };
    // SAFETY: `limit` is a valid rlimit that outlives the call.
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &raw const limit) } != 0 {
        return Err(SandboxError::last_os("setrlimit(RLIMIT_AS)"));
    }
    #[cfg(feature = "seccomp-trap")]
    trap::install();
    for filter in filters()? {
        seccompiler::apply_filter_all_threads(&filter)?;
    }
    let mut layers = vec![Layer::NoNewPrivs];
    layers.extend(landlock);
    layers.extend([Layer::MemoryLimit, Layer::Seccomp]);
    Ok(SandboxLevel { layers })
}

/// How many descriptors above 2 the engine holds: the main process starts it with every other
/// one closed (`spawn.rs`), so any is a leak. Read from /proc before seccomp forbids it.
fn inherited_descriptors() -> Result<usize, SandboxError> {
    let entries = std::fs::read_dir("/proc/self/fd").map_err(|source| SandboxError::Os {
        step: "listing /proc/self/fd",
        source,
    })?;
    let above = entries
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<i32>().ok())
        .filter(|fd| *fd > 2)
        .count();
    // The listing itself holds one.
    Ok(above.saturating_sub(1))
}

/// Applies the deny-all Landlock ruleset; returns the layer the kernel enforced, if any. Best
/// effort: a failure or an old kernel is logged, not fatal, since seccomp still denies every path.
fn landlock() -> Option<Layer> {
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(LANDLOCK_ABI))
        .and_then(|r| r.handle_access(AccessNet::from_all(LANDLOCK_ABI)))
        .and_then(|r| r.scope(Scope::from_all(LANDLOCK_ABI)))
        .and_then(Ruleset::create)
        .and_then(RulesetCreated::restrict_self);
    match ruleset {
        Ok(status) => {
            let abi = match status.landlock {
                LandlockStatus::Available { effective_abi, .. } => effective_abi as u8,
                _ => 0,
            };
            match status.ruleset {
                RulesetStatus::NotEnforced => {
                    super::log("Landlock is off: the kernel doesn't enforce it");
                    None
                }
                RulesetStatus::PartiallyEnforced => Some(Layer::Landlock { abi, full: false }),
                RulesetStatus::FullyEnforced => Some(Layer::Landlock { abi, full: true }),
            }
        }
        Err(error) => {
            super::log(&format!("Landlock is off: {error}"));
            None
        }
    }
}

#[cfg(target_arch = "x86_64")]
const ARCH: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCH: TargetArch = TargetArch::aarch64;

/// The calls the engine makes once locked down, whatever their arguments.
const ALLOWED: &[libc::c_long] = &[
    // Its pipes and its log.
    libc::SYS_read,
    libc::SYS_write,
    libc::SYS_readv,
    libc::SYS_writev,
    libc::SYS_close,
    // Memory (`mmap` and `mprotect` have their own rules: no `PROT_EXEC`).
    libc::SYS_munmap,
    libc::SYS_mremap,
    libc::SYS_madvise,
    libc::SYS_brk,
    // Threads and waiting.
    libc::SYS_futex,
    libc::SYS_set_robust_list,
    libc::SYS_rseq,
    libc::SYS_set_tid_address,
    libc::SYS_sched_yield,
    libc::SYS_sched_getaffinity,
    libc::SYS_nanosleep,
    libc::SYS_clock_nanosleep,
    libc::SYS_clock_gettime,
    libc::SYS_clock_getres,
    libc::SYS_gettimeofday,
    libc::SYS_getpid,
    libc::SYS_gettid,
    libc::SYS_getrandom,
    // Signals: handlers, masks and the alternate stack std gives each thread.
    libc::SYS_rt_sigaction,
    libc::SYS_rt_sigprocmask,
    libc::SYS_rt_sigreturn,
    libc::SYS_sigaltstack,
    libc::SYS_exit,
    libc::SYS_exit_group,
    // Allowed here, answered by the later filters.
    libc::SYS_clone3,
    libc::SYS_getrlimit,
];

/// The calls that name a path: `EACCES`, so a lookup fails like a missing file. And `sysinfo`
/// (uptime, load, memory), which glibc's `qsort` asks for and does without.
const PATHS: &[libc::c_long] = &[
    libc::SYS_sysinfo,
    libc::SYS_openat,
    libc::SYS_openat2,
    libc::SYS_newfstatat,
    libc::SYS_statx,
    libc::SYS_faccessat,
    libc::SYS_faccessat2,
    libc::SYS_readlinkat,
    libc::SYS_mkdirat,
    libc::SYS_unlinkat,
    libc::SYS_renameat2,
    #[cfg(target_arch = "x86_64")]
    libc::SYS_open,
    #[cfg(target_arch = "x86_64")]
    libc::SYS_stat,
    #[cfg(target_arch = "x86_64")]
    libc::SYS_lstat,
    #[cfg(target_arch = "x86_64")]
    libc::SYS_access,
    #[cfg(target_arch = "x86_64")]
    libc::SYS_readlink,
];

/// The three filters, in the order they are installed. Which filter comes first doesn't change
/// the answer: the kernel takes the strictest.
fn filters() -> Result<Vec<BpfProgram>, seccompiler::Error> {
    let pid = u64::from(std::process::id());
    let mut allow: BTreeMap<i64, Vec<SeccompRule>> = ALLOWED.iter().chain(PATHS).map(|&nr| (nr, Vec::new())).collect();
    let no_exec = || masked(2, libc::PROT_EXEC as u64, 0);
    allow.insert(libc::SYS_mmap, vec![rule(vec![no_exec()?])?]);
    allow.insert(libc::SYS_mprotect, vec![rule(vec![no_exec()?])?]);
    let thread = libc::CLONE_THREAD as u64;
    allow.insert(libc::SYS_clone, vec![rule(vec![masked(0, thread, thread)?])?]);
    // Names: std names its threads through prctl, glibc its anonymous mappings.
    allow.insert(
        libc::SYS_prctl,
        vec![
            rule(vec![equal(0, libc::PR_SET_NAME as u64)?])?,
            rule(vec![equal(0, libc::PR_GET_NAME as u64)?])?,
            // glibc (2.39 and later) names its malloc arenas and thread stacks.
            rule(vec![equal(0, PR_SET_VMA)?])?,
        ],
    );
    // The selftest's descriptor probe: whether a descriptor is open, nothing else. Test builds only.
    #[cfg(feature = "probe")]
    allow.insert(libc::SYS_fcntl, vec![rule(vec![equal(1, libc::F_GETFD as u64)?])?]);
    // Reading a limit, never setting one.
    allow.insert(libc::SYS_prlimit64, vec![rule(vec![equal(2, 0)?])?]);
    // `abort` signals the process itself; nothing else.
    allow.insert(libc::SYS_tgkill, vec![rule(vec![equal(0, pid)?])?]);
    let paths = PATHS.iter().map(|&nr| (nr, Vec::new())).collect();
    let clone3 = BTreeMap::from([(libc::SYS_clone3, Vec::new())]);
    let mismatch = if cfg!(feature = "seccomp-trap") {
        SeccompAction::Trap
    } else {
        SeccompAction::KillProcess
    };
    // The allow-list goes last: installing a filter calls prctl and seccomp, which it forbids.
    [
        (paths, SeccompAction::Allow, SeccompAction::Errno(libc::EACCES as u32)),
        (clone3, SeccompAction::Allow, SeccompAction::Errno(libc::ENOSYS as u32)),
        (allow, mismatch, SeccompAction::Allow),
    ]
    .into_iter()
    .map(|(rules, mismatch, matched)| {
        let filter = SeccompFilter::new(rules, mismatch, matched, ARCH)?;
        Ok(BpfProgram::try_from(filter)?)
    })
    .collect()
}

fn rule(conditions: Vec<SeccompCondition>) -> Result<SeccompRule, seccompiler::Error> {
    Ok(SeccompRule::new(conditions)?)
}

/// Argument `arg` (all 64 bits) equals `value`.
fn equal(arg: u8, value: u64) -> Result<SeccompCondition, seccompiler::Error> {
    Ok(SeccompCondition::new(
        arg,
        SeccompCmpArgLen::Qword,
        SeccompCmpOp::Eq,
        value,
    )?)
}

/// Argument `arg` masked with `mask` equals `value`.
fn masked(arg: u8, mask: u64, value: u64) -> Result<SeccompCondition, seccompiler::Error> {
    Ok(SeccompCondition::new(
        arg,
        SeccompCmpArgLen::Qword,
        SeccompCmpOp::MaskedEq(mask),
        value,
    )?)
}

/// The `seccomp-trap` feature's SIGSYS handler: names the call the filter refused.
#[cfg(feature = "seccomp-trap")]
mod trap {
    /// Where `siginfo_t` keeps a SIGSYS's system call number on Linux (`_sigsys._syscall`: after
    /// the signal number, errno and code, padded to 16 bytes, then the call's address).
    const SYSCALL_OFFSET: usize = 16 + size_of::<usize>();

    extern "C" fn on_sigsys(_: libc::c_int, info: *mut libc::siginfo_t, _: *mut libc::c_void) {
        // SAFETY: the kernel passes a valid siginfo_t for a SA_SIGINFO handler; for SIGSYS the
        // call number is an i32 at SYSCALL_OFFSET, inside the 128-byte structure.
        let nr = unsafe { info.cast::<u8>().add(SYSCALL_OFFSET).cast::<i32>().read_unaligned() };
        let mut line = *b"seccomp refused system call       \n";
        let mut at = line.len() - 2;
        let mut n = nr.unsigned_abs();
        loop {
            if let Some(digit) = line.get_mut(at) {
                *digit = b'0' + (n % 10) as u8;
            }
            n /= 10;
            if n == 0 || at == 0 {
                break;
            }
            at -= 1;
        }
        // SAFETY: write and _exit are async-signal-safe; `line` is a valid buffer of its length.
        unsafe {
            libc::write(2, line.as_ptr().cast(), line.len());
            libc::_exit(159);
        }
    }

    pub(super) fn install() {
        // SAFETY: a zeroed sigaction is valid; the handler has the SA_SIGINFO signature and only
        // calls async-signal-safe functions.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = on_sigsys as *const () as libc::sighandler_t;
            action.sa_flags = libc::SA_SIGINFO;
            libc::sigaction(libc::SIGSYS, &raw const action, std::ptr::null_mut());
        }
    }
}
