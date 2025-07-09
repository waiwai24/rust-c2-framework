use std::io::BufRead;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io};

const MIN_UPTIME_SECONDS: u64 = 3600; // 1 hour
const MAX_TEMP_FILES: usize = 10; // Arbitrary threshold
const SLEEP_DURATION_SECONDS: u64 = 10;
const SLEEP_ACCURACY_SECONDS: u64 = 2; // Allow 2 seconds of inaccuracy

/// Runs all anti-sandbox and anti-debugging checks.
/// Returns true if a sandbox or debugger is detected, false otherwise.
pub fn run_all_checks() -> bool {
    is_debugger_present() || is_in_sandbox()
}

/// Checks for signs of being in a sandbox environment.
pub fn is_in_sandbox() -> bool {
    check_delay_execution() || check_uptime() || check_temp_files()
}

/// Checks for the presence of a debugger.
/// On Linux, this checks the TracerPid in /proc/self/status.
pub fn is_debugger_present() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(file) = fs::File::open("/proc/self/status") {
            let reader = io::BufReader::new(file);

            for line in reader.lines().filter_map(Result::ok) {
                if let Some(pid) = line.strip_prefix("TracerPid:") {
                    return pid.trim().parse().map(|n: u32| n != 0).unwrap_or(false);
                }
            }
        }
        unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) == -1 }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Detects sandboxes that fast-forward time during sleep.
fn check_delay_execution() -> bool {
    let start = Instant::now();
    thread::sleep(Duration::from_secs(SLEEP_DURATION_SECONDS));
    let elapsed = start.elapsed().as_secs();

    // If the elapsed time is significantly less than the sleep duration,
    // it's a sign of a sandbox manipulating time.
    elapsed < (SLEEP_DURATION_SECONDS - SLEEP_ACCURACY_SECONDS)
}

/// Checks the system uptime. Low uptime can indicate a sandbox.
/// On Linux, reads /proc/uptime.
fn check_uptime() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
            if let Some(uptime_val_str) = uptime_str.split_whitespace().next() {
                if let Ok(uptime_secs) = uptime_val_str.parse::<f64>() {
                    return (uptime_secs as u64) < MIN_UPTIME_SECONDS;
                }
            }
        }
    }
    // Default to not detected if uptime can't be checked
    false
}

/// Checks the number of files in the temporary directory.
/// A very low count might indicate a clean sandbox environment.
fn check_temp_files() -> bool {
    let temp_dir = std::env::temp_dir();
    if let Ok(entries) = fs::read_dir(temp_dir) {
        let count = entries.count();
        return count < MAX_TEMP_FILES;
    }
    // Default to not detected if temp dir can't be read
    false
}
