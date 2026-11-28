// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Optional macOS sidecar for local host diagnostic evidence.

use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    os::{
        fd::{FromRawFd, OwnedFd},
        unix::{fs::OpenOptionsExt, net::UnixDatagram},
    },
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use crate::host_watchdog::{HostObservation, HostObservationRecord, HostObserver, HostPulse};

const MODE: &str = "--marlin-internal-host-observer";
const THRESHOLD: Duration = Duration::from_secs(2);

fn is_macos_system_root_link(dir: &Path) -> bool {
    dir == Path::new("/var") || dir == Path::new("/tmp") || dir == Path::new("/etc")
}

fn verify_report_path_no_symlinks(path: &Path) -> io::Result<()> {
    let raw = path.as_os_str();
    if raw.is_empty() || path.to_string_lossy().chars().any(char::is_control) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "host observer report path must be nonempty without control characters",
        ));
    }
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir.as_os_str().is_empty() || dir == Path::new("/") {
            break;
        }
        if !is_macos_system_root_link(dir)
            && let Ok(meta) = std::fs::symlink_metadata(dir)
            && meta.file_type().is_symlink()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "host observer report path traverses a symlinked directory",
            ));
        }
        cur = dir.parent();
    }
    Ok(())
}

/// Start the observer when a local report file is explicitly configured.
pub fn start_host_observer(path: &Path) -> io::Result<HostPulse> {
    verify_report_path_no_symlinks(path)?;
    let report = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    let (send, receive) = HostPulse::channel()?;
    let mut child = Command::new(env::current_exe()?)
        .env_clear()
        .arg(MODE)
        .arg(std::process::id().to_string())
        .stdin(Stdio::from(OwnedFd::from(receive)))
        .stdout(Stdio::from(report))
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    // Reap early observer failures while the host is still running. On host
    // exit the observer receives NOTE_EXIT and its new parent reaps it.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(send)
}

use std::os::unix::process::CommandExt;

/// Dispatch the private observer mode before the host initializes the UI or telemetry.
pub fn run_host_observer_if_requested() -> bool {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(MODE)) {
        return false;
    }
    let host_pid = args
        .next()
        .and_then(|arg| arg.to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid > 1);
    let extra_args = args.next().is_some();
    let result = if extra_args {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected trailing observer arguments",
        ))
    } else {
        host_pid
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "invalid observer host PID")
            })
            .and_then(run)
    };
    if result.is_err() {
        std::process::exit(1);
    }
    true
}

fn run(host_pid: u32) -> io::Result<()> {
    // SAFETY: getppid reports this process's current parent.
    let current_parent = || unsafe { libc::getppid() as u32 };
    if current_parent() != host_pid {
        return Err(io::Error::other(
            "observer parent changed before registration",
        ));
    }
    // SAFETY: fcntl validates descriptor 0 before ownership is taken.
    if unsafe { libc::fcntl(0, libc::F_GETFD) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the private invocation owns fd 0, supplied as a Unix datagram socket.
    let socket = UnixDatagram::from(unsafe { OwnedFd::from_raw_fd(0) });
    let mut observer = HostObserver::attach(host_pid, socket, THRESHOLD)?;
    if current_parent() != host_pid {
        return Err(io::Error::other(
            "observer parent changed during registration",
        ));
    }
    let mut output = io::stdout().lock();
    let mut budget: usize = 1024;
    loop {
        let mut written = Ok(());
        let running = observer.sample(&mut |observation| {
            if written.is_ok() && (budget != 0 || observation == HostObservation::Exited) {
                budget = budget.saturating_sub(1);
                written = serde_json::to_writer(
                    &mut output,
                    &HostObservationRecord {
                        host_pid,
                        observation,
                    },
                )
                .map_err(io::Error::other)
                .and_then(|()| output.write_all(b"\n"))
                .and_then(|()| output.flush());
            }
        })?;
        written?;
        if !running {
            return Ok(());
        }
    }
}
