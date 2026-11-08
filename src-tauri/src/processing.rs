//! Bound untrusted media processing. No shell, inherited credentials, or network sockets.
use std::{
    fs::File,
    io::{self, Read},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn command(program: &str, input: File) -> Command {
    let mut command = Command::new(format!("/usr/bin/{program}"));
    command
        .env_clear()
        .env("PATH", "/usr/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let filter = network_filter();
    let parent = std::process::id() as i32;
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(libc::ECHILD));
            }
            for (resource, limit) in [
                (libc::RLIMIT_CORE, 0),
                (libc::RLIMIT_AS, 2 * 1024 * 1024 * 1024),
                (libc::RLIMIT_CPU, 600),
                (libc::RLIMIT_FSIZE, 1024 * 1024 * 1024),
            ] {
                let mut existing = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::getrlimit(resource, &mut existing) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let limit = limit.min(existing.rlim_max);
                let limit = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(resource, &limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            // A separate process group lets timeout cleanup include descendants.
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            install_filter(&filter)
        });
    }
    command
}
fn network_filter() -> Vec<libc::sock_filter> {
    // The distributed package targets x86_64; reject other syscall architectures
    // and the x32 syscall namespace so the filter cannot be bypassed through them.
    let stmt = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k, jt, jf| libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    };
    let mut filter = vec![
        stmt(0x20, 4),
        jump(0xc000003e, 1, 0),
        stmt(0x06, 0x80000000),
        stmt(0x20, 0),
        libc::sock_filter {
            code: 0x35,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        },
        stmt(0x06, 0x80000000),
    ];
    for syscall in [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_connect,
        libc::SYS_bind,
        libc::SYS_listen,
        libc::SYS_accept,
        libc::SYS_accept4,
        libc::SYS_sendto,
        libc::SYS_sendmsg,
        libc::SYS_sendmmsg,
        libc::SYS_recvfrom,
        libc::SYS_recvmsg,
        libc::SYS_recvmmsg,
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_pidfd_getfd,
        libc::SYS_io_uring_setup,
        libc::SYS_fork,
        libc::SYS_vfork,
    ] {
        filter.push(jump(syscall as u32, 0, 1));
        filter.push(stmt(0x06, 0x00050000 | libc::EPERM as u32));
    }
    // glibc retries clone when clone3 is unavailable. Allow thread creation only.
    filter.push(jump(libc::SYS_clone3 as u32, 0, 1));
    filter.push(stmt(0x06, 0x00050000 | libc::ENOSYS as u32));
    filter.push(jump(libc::SYS_clone as u32, 0, 4));
    filter.push(stmt(0x20, 16));
    filter.push(libc::sock_filter {
        code: 0x45,
        jt: 1,
        jf: 0,
        k: libc::CLONE_THREAD as u32,
    });
    filter.push(stmt(0x06, 0x00050000 | libc::EPERM as u32));
    filter.push(stmt(0x06, 0x7fff0000));
    filter.push(stmt(0x06, 0x7fff0000));
    filter
}
fn install_filter(filter: &[libc::sock_filter]) -> io::Result<()> {
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_ptr() as *mut libc::sock_filter,
    };
    unsafe {
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            || libc::prctl(libc::PR_SET_SECCOMP, 2, &program) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
fn bounded_read(mut stream: impl Read, limit: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut buf = [0; 4096];
    while let Ok(count) = stream.read(&mut buf) {
        if count == 0 {
            break;
        }
        let keep = count.min(limit.saturating_sub(data.len()));
        data.extend_from_slice(&buf[..keep]);
    }
    data
}
pub fn run(mut command: Command, timeout: Duration) -> Result<Vec<u8>, String> {
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start secure video processing: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out = std::thread::spawn(move || stdout.map(|s| bounded_read(s, 4096)).unwrap_or_default());
    let err =
        std::thread::spawn(move || stderr.map(|s| bounded_read(s, 65536)).unwrap_or_default());
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            result => {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                break Err(match result {
                    Err(e) => e.to_string(),
                    _ => "Video processing timed out".into(),
                });
            }
        }
    };
    let data = out.join().unwrap_or_default();
    let error = err.join().unwrap_or_default();
    let status = status?;
    if !status.success() {
        return Err(format!(
            "Video processing failed: {}",
            String::from_utf8_lossy(&error)
                .chars()
                .take(1000)
                .collect::<String>()
        ));
    }
    Ok(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn processing_cannot_open_network_sockets() {
        let mut cmd = command("python3", File::open("/dev/null").unwrap());
        cmd.args(["-c","import socket,os; assert 'HOME' not in os.environ\ntry: socket.socket()\nexcept PermissionError: print('blocked')\nelse: raise RuntimeError('socket allowed')"]);
        assert_eq!(run(cmd, Duration::from_secs(3)).unwrap(), b"blocked\n");
    }
    #[test]
    fn processing_cannot_fork_children() {
        let mut cmd = command("python3", File::open("/dev/null").unwrap());
        cmd.args(["-c","import os\ntry: os.fork()\nexcept PermissionError: print('blocked')\nelse: raise RuntimeError('fork allowed')"]);
        assert_eq!(run(cmd, Duration::from_secs(3)).unwrap(), b"blocked\n");
    }
    #[test]
    fn hung_jobs_are_terminated() {
        let mut cmd = command("sleep", File::open("/dev/null").unwrap());
        cmd.arg("30");
        let start = Instant::now();
        assert!(run(cmd, Duration::from_millis(50))
            .unwrap_err()
            .contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn job_output_is_bounded() {
        let mut cmd = command("python3", File::open("/dev/null").unwrap());
        cmd.args(["-c", "print('x'*1000000)"]);
        assert_eq!(run(cmd, Duration::from_secs(3)).unwrap().len(), 4096);
    }
}
