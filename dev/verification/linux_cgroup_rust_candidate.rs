// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Disposable Linux experiment only. Not a production isolation backend.
//! Compiled as a standalone Linux binary; run inside a root-owned delegated
//! systemd scope. It launches only the hard-coded test fixture below.

use std::{
    ffi::{CString, c_char, c_int, c_ulong},
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

unsafe extern "C" {
    fn geteuid() -> u32;
    fn setgroups(size: usize, list: *const u32) -> c_int;
    fn setgid(gid: u32) -> c_int;
    fn setuid(uid: u32) -> c_int;
    fn getresuid(real: *mut u32, effective: *mut u32, saved: *mut u32) -> c_int;
    fn getresgid(real: *mut u32, effective: *mut u32, saved: *mut u32) -> c_int;
    fn syscall(number: i64, ...) -> i64;
    fn prctl(option: c_int, arg2: c_ulong, arg3: c_ulong, arg4: c_ulong, arg5: c_ulong) -> c_int;
    fn write(fd: c_int, buffer: *const u8, count: usize) -> isize;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    fn poll(fds: *mut PollFd, count: usize, timeout_ms: c_int) -> c_int;
}

const PR_SET_NO_NEW_PRIVS: c_int = 38;
const PR_GET_NO_NEW_PRIVS: c_int = 39;
#[cfg(target_arch = "aarch64")]
const SYS_CAPGET: i64 = 90;
#[cfg(not(target_arch = "aarch64"))]
compile_error!("this qualification fixture is specific to aarch64 Linux");

#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}
const O_WRONLY: c_int = 1;
const O_CLOEXEC: c_int = 0o2000000;
const EACCES: i32 = 13;
const EPERM: i32 = 1;
const EROFS: i32 = 30;
const POLLIN: i16 = 1;
#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

fn require(condition: bool, message: &'static str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

fn scope() -> io::Result<PathBuf> {
    // SAFETY: geteuid has no preconditions.
    require(unsafe { geteuid() } == 0, "requires root")?;
    let membership = fs::read_to_string("/proc/self/cgroup")?;
    let relative = membership
        .trim()
        .strip_prefix("0::/")
        .ok_or_else(|| io::Error::other("requires cgroup v2"))?;
    let path = Path::new("/sys/fs/cgroup").join(relative);
    let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    require(
        name.starts_with("run-") && name.ends_with(".scope"),
        "requires a disposable systemd scope",
    )?;
    require(path.join("cgroup.kill").exists(), "requires cgroup.kill")?;
    Ok(path)
}

fn populated(group: &Path) -> io::Result<bool> {
    let contents = fs::read_to_string(group.join("cgroup.events"))?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("populated "))
        .map(|value| value == "1")
        .ok_or_else(|| io::Error::other("missing populated status"))
}

fn wait_empty(group: &Path) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while populated(group)? {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "cgroup still populated",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let parent = scope()?;
    let group = parent.join(format!("marlin-rust-qualification-{}", std::process::id()));
    fs::create_dir(&group)?;
    let result = exercise(&parent, &group);
    // Restrict all cleanup to the unique child cgroup owned by this fixture.
    let _ = fs::write(group.join("cgroup.kill"), "1");
    let cleanup = wait_empty(&group).and_then(|_| fs::remove_dir(&group));
    result.and(cleanup)
}

fn exercise(parent: &Path, group: &Path) -> io::Result<()> {
    let membership = OpenOptions::new()
        .write(true)
        .open(group.join("cgroup.procs"))?;
    let group_fd = membership.as_raw_fd();
    let cgroup_root = Path::new("/sys/fs/cgroup");
    let mut ancestors = Vec::new();
    let mut cursor = parent;
    loop {
        ancestors.push(
            CString::new(cursor.join("cgroup.procs").as_os_str().as_bytes())
                .map_err(|_| io::Error::other("NUL in cgroup path"))?,
        );
        if cursor == cgroup_root {
            break;
        }
        cursor = cursor
            .parent()
            .ok_or_else(|| io::Error::other("cgroup root not an ancestor"))?;
    }
    let script = r#"import os,sys,time
os.setsid()
try:
    open(sys.argv[1], 'w').write(str(os.getpid()))
    denied = 0
except PermissionError:
    denied = 1
p = os.fork()
if p == 0:
    os.setsid()
    time.sleep(30)
    os._exit(0)
s = open('/proc/self/status').read()
caps = int('CapEff:\\t0000000000000000' in s and 'CapPrm:\\t0000000000000000' in s and 'NoNewPrivs:\\t1' in s)
print(os.getpid(), p, os.geteuid(), denied, caps, flush=True)
time.sleep(30)
"#.replace("\\\\t", "\\t");
    let mut fds = [-1; 2];
    // SAFETY: fds is a writable two-element array; on success both descriptors
    // are newly owned, and immediately wrapped in File.
    if unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let read_end = unsafe { File::from_raw_fd(fds[0]) };
    let write_end = unsafe { File::from_raw_fd(fds[1]) };
    let external_writer = write_end.try_clone()?;
    let mut command = Command::new("/usr/bin/python3");
    command
        .arg("-c")
        .arg(script)
        .arg(parent.join("cgroup.procs"))
        .stdin(Stdio::null())
        .stdout(Stdio::from(write_end))
        .stderr(Stdio::piped());
    // SAFETY: this callback performs only direct system calls and constructs
    // OS errors; it does not allocate, acquire locks, or invoke target code.
    unsafe {
        command.pre_exec(move || {
            if write(group_fd, b"0".as_ptr(), 1) != 1 {
                return Err(io::Error::last_os_error());
            }
            if setgroups(0, std::ptr::null()) != 0 || setgid(65534) != 0 || setuid(65534) != 0 {
                return Err(io::Error::last_os_error());
            }
            if prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            let (mut ruid, mut euid, mut suid) = (0, 0, 0);
            let (mut rgid, mut egid, mut sgid) = (0, 0, 0);
            if getresuid(&mut ruid, &mut euid, &mut suid) != 0
                || getresgid(&mut rgid, &mut egid, &mut sgid) != 0
                || (ruid, euid, suid) != (65534, 65534, 65534)
                || (rgid, egid, sgid) != (65534, 65534, 65534)
                || prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 1
            {
                return Err(io::Error::from_raw_os_error(EPERM));
            }
            let mut header = CapHeader {
                version: 0x20080522,
                pid: 0,
            };
            let mut caps = [CapData {
                effective: 0,
                permitted: 0,
                inheritable: 0,
            }; 2];
            if syscall(SYS_CAPGET, &mut header as *mut CapHeader, caps.as_mut_ptr()) != 0
                || caps
                    .iter()
                    .any(|cap| cap.effective != 0 || cap.permitted != 0 || cap.inheritable != 0)
            {
                return Err(io::Error::from_raw_os_error(EPERM));
            }
            for path in &ancestors {
                let fd = open(path.as_ptr(), O_WRONLY | O_CLOEXEC);
                if fd >= 0 {
                    close(fd);
                    return Err(io::Error::from_raw_os_error(EPERM));
                }
                let error = io::Error::last_os_error();
                if !matches!(error.raw_os_error(), Some(EACCES | EPERM | EROFS)) {
                    return Err(error);
                }
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    drop(command);
    drop(membership);
    let mut output = BufReader::new(read_end);
    let mut line = String::new();
    output.read_line(&mut line)?;
    let words: Vec<_> = line.split_whitespace().collect();
    require(
        words.len() == 5 && words[2..] == ["65534", "1", "1"],
        "unexpected child authority receipt",
    )?;
    let leader = words[0];
    let descendant = words[1];
    let members = fs::read_to_string(group.join("cgroup.procs"))?;
    require(
        members.lines().any(|pid| pid == leader) && members.lines().any(|pid| pid == descendant),
        "leader or setsid descendant outside cgroup",
    )?;
    fs::write(group.join("cgroup.kill"), "1")?;
    let _status = child.wait()?;
    wait_empty(group)?;
    let mut descriptor = PollFd {
        fd: output.get_ref().as_raw_fd(),
        events: POLLIN,
        revents: 0,
    };
    // SAFETY: descriptor points to a valid pollfd for the duration of poll.
    let result = unsafe { poll(&mut descriptor, 1, 100) };
    require(result == 0, "external writer failed to keep pipe open")?;
    drop(external_writer);
    let mut rest = Vec::new();
    output.read_to_end(&mut rest)?;
    require(rest.is_empty(), "unexpected trailing stdout")?;
    println!(
        "RUST_CGROUP_CANDIDATE_OK: restricted setsid descendant contained; empty group; external writer delayed EOF"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("RUST_CGROUP_CANDIDATE_ERROR: {error}");
        std::process::exit(1);
    }
}
