//! The machine a measurement ran on: operating system and version, CPU model, core count,
//! memory and the load average when the run started. The benchmarks page names it beside every
//! figure, since timings from a shared, busy machine read differently from an idle one's.
//!
//! macOS answers through `sysctl` and `sw_vers`, Linux through `/proc/cpuinfo`, `/proc/meminfo`,
//! `/proc/loadavg` and `uname`. A value the machine doesn't give is `None` (`null` in JSON), never
//! a guess.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

/// What [`describe`] found.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Machine {
    /// `macOS 27.0.1`, `Linux 6.8.0`.
    pub(crate) os: Option<String>,
    /// `Apple M1 Pro`.
    pub(crate) cpu: Option<String>,
    /// Logical cores.
    pub(crate) cores: Option<u32>,
    pub(crate) memory_bytes: Option<u64>,
    /// The one-minute load average.
    pub(crate) load_average: Option<f64>,
}

impl Machine {
    /// The JSON the benchmark files hold: `{ os, cpu, cores, memoryBytes, loadAverage }`.
    pub(crate) fn json(&self) -> Value {
        json!({
            "os": self.os,
            "cpu": self.cpu,
            "cores": self.cores,
            "memoryBytes": self.memory_bytes,
            "loadAverage": self.load_average,
        })
    }
}

/// What a benchmark section says about its run: Lamina PDF's version, the commit, the date (UTC)
/// and [`describe`]'s machine, as `{ version, commit, date, machine }`.
pub(crate) fn stamp(root: &Path) -> Value {
    let version = crate::version::fixed_group_version(root).map(|g| g.version).ok();
    json!({
        "version": version,
        "commit": crate::util::git_stdout(root, &["rev-parse", "HEAD"]),
        "date": crate::progress::today(),
        "machine": describe().json(),
    })
}

/// This machine, now.
pub(crate) fn describe() -> Machine {
    if cfg!(target_os = "macos") {
        let sysctl = |key: &str| stdout("sysctl", &["-n", key]);
        Machine {
            os: stdout("sw_vers", &["-productVersion"]).map(|v| format!("macOS {}", v.trim())),
            cpu: sysctl("machdep.cpu.brand_string").map(|c| c.trim().to_owned()),
            cores: sysctl("hw.ncpu").and_then(|n| n.trim().parse().ok()),
            memory_bytes: sysctl("hw.memsize").and_then(|n| n.trim().parse().ok()),
            load_average: sysctl("vm.loadavg").as_deref().and_then(macos_load_average),
        }
    } else {
        let read = |path: &str| std::fs::read_to_string(Path::new(path)).ok();
        let cpuinfo = read("/proc/cpuinfo");
        Machine {
            os: stdout("uname", &["-sr"]).map(|v| v.trim().to_owned()),
            cpu: cpuinfo.as_deref().and_then(linux_cpu_model),
            cores: cpuinfo.as_deref().map(linux_cores).filter(|&n| n > 0),
            memory_bytes: read("/proc/meminfo").as_deref().and_then(linux_memory_bytes),
            load_average: read("/proc/loadavg").as_deref().and_then(first_number),
        }
    }
}

/// A command's standard output, or `None` when it fails or can't run.
fn stdout(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The first number of `sysctl -n vm.loadavg`: `{ 1.52 1.80 2.03 }`.
fn macos_load_average(text: &str) -> Option<f64> {
    first_number(text.trim().trim_start_matches('{'))
}

/// The first whitespace-separated number in `text` (`/proc/loadavg` starts with the one-minute
/// average).
fn first_number(text: &str) -> Option<f64> {
    text.split_whitespace().next()?.parse().ok()
}

/// `/proc/cpuinfo`'s `model name` (x86), else its `Hardware` or `Model` line (some Arm boards);
/// many Arm kernels give none of them.
fn linux_cpu_model(cpuinfo: &str) -> Option<String> {
    ["model name", "Hardware", "Model"].iter().find_map(|key| {
        cpuinfo.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == *key && !v.trim().is_empty()).then(|| v.trim().to_owned())
        })
    })
}

/// How many `processor` entries `/proc/cpuinfo` lists.
fn linux_cores(cpuinfo: &str) -> u32 {
    let n = cpuinfo
        .lines()
        .filter(|l| l.split_once(':').is_some_and(|(k, _)| k.trim() == "processor"))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// `/proc/meminfo`'s `MemTotal`, which it gives in kibibytes.
fn linux_memory_bytes(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kib: u64 = line
        .trim_start_matches("MemTotal:")
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    kib.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_load_average_should_read_the_one_minute_figure() {
        assert_eq!(macos_load_average("{ 160.36 92.46 59.85 }\n"), Some(160.36));
    }

    #[test]
    fn macos_load_average_should_be_none_for_unexpected_text() {
        assert_eq!(macos_load_average("unknown oid\n"), None);
    }

    const CPUINFO_X86: &str = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) CPU @ 2.20GHz\n\n\
                               processor\t: 1\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Xeon(R) CPU @ 2.20GHz\n";
    const CPUINFO_ARM: &str = "processor\t: 0\nBogoMIPS\t: 48.00\nCPU part\t: 0xd0c\n\nprocessor\t: 1\nBogoMIPS\t: 48.00\n\n\
                               processor\t: 2\n\nprocessor\t: 3\n";

    #[test]
    fn linux_cpu_model_should_read_the_model_name() {
        assert_eq!(
            linux_cpu_model(CPUINFO_X86).as_deref(),
            Some("Intel(R) Xeon(R) CPU @ 2.20GHz")
        );
    }

    #[test]
    fn linux_cpu_model_should_be_none_when_the_cpuinfo_names_no_model() {
        assert_eq!(linux_cpu_model(CPUINFO_ARM), None);
    }

    #[test]
    fn linux_cores_should_count_the_processor_entries() {
        assert_eq!((linux_cores(CPUINFO_X86), linux_cores(CPUINFO_ARM)), (2, 4));
    }

    #[test]
    fn linux_memory_bytes_should_turn_mem_total_kibibytes_into_bytes() {
        let meminfo = "MemTotal:       16384000 kB\nMemFree:         1000000 kB\n";
        assert_eq!(linux_memory_bytes(meminfo), Some(16_384_000 * 1024));
    }

    #[test]
    fn linux_load_average_should_read_the_first_field() {
        assert_eq!(first_number("0.52 0.58 0.59 1/389 12345\n"), Some(0.52));
    }

    #[test]
    fn json_should_name_every_field_and_write_null_for_the_unknown() {
        let m = Machine {
            os: Some("macOS 27.0.1".into()),
            cpu: None,
            cores: Some(8),
            memory_bytes: None,
            load_average: Some(1.5),
        };
        assert_eq!(
            m.json(),
            json!({"os": "macOS 27.0.1", "cpu": null, "cores": 8, "memoryBytes": null, "loadAverage": 1.5})
        );
    }
}
