// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Optional task and process observer outside the terminal event loop.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Hard cumulative ceiling on runtime diagnostic probe file size (4 MiB, SM39).
pub(crate) const MAX_RUNTIME_PROBE_BYTES: usize = 4 * 1024 * 1024;

fn open_confined_probe_file(path: &Path) -> io::Result<File> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime probe path must not be empty",
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && std::fs::symlink_metadata(parent)
            .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime probe parent directory must not be a symlink",
        ));
    }
    if std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime probe path must not be a symlink",
        ));
    }
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW).mode(0o600);
    }
    opts.open(path)
}

// Optional diagnostic observer. It runs outside Tokio so it does not inflate
// the runtime task count and never performs file I/O on the UI thread.
pub(crate) struct RuntimeProbe(Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>);

impl RuntimeProbe {
    pub(crate) fn start(
        runtime: &tokio::runtime::Runtime,
        owner: &marlin_agent_runtime::TokioAgentRuntime,
    ) -> Self {
        let Some(path) = std::env::var_os("MARLIN_TUI_RUNTIME_METRICS_FILE") else {
            return Self(None);
        };
        Self::start_to_path(runtime, owner, PathBuf::from(path), MAX_RUNTIME_PROBE_BYTES)
    }

    pub(crate) fn start_to_path(
        runtime: &tokio::runtime::Runtime,
        owner: &marlin_agent_runtime::TokioAgentRuntime,
        path: PathBuf,
        max_bytes: usize,
    ) -> Self {
        let metrics = runtime.handle().metrics();
        let tracker = owner.task_tracker();
        let processes = owner.process_registry();
        let (stop, receiver) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let Ok(mut file) = open_confined_probe_file(&path) else {
                return;
            };
            let mut written_bytes: usize = 0;
            let mut stopped = false;
            loop {
                let row = format!(
                    "{},{},{},{}\n",
                    metrics.num_alive_tasks(),
                    metrics.num_workers(),
                    tracker.active_task_count(),
                    processes.active_processes().len()
                );
                if written_bytes.saturating_add(row.len()) > max_bytes
                    || file.write_all(row.as_bytes()).is_err()
                {
                    return;
                }
                written_bytes = written_bytes.saturating_add(row.len());
                if stopped {
                    return;
                }
                // Capture the final state after Runtime drain when the owner stops the probe.
                stopped = !matches!(
                    receiver.recv_timeout(std::time::Duration::from_millis(250)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                );
            }
        });
        Self(Some((stop, thread)))
    }
}

impl Drop for RuntimeProbe {
    fn drop(&mut self) {
        if let Some((stop, thread)) = self.0.take() {
            let _ = stop.send(());
            let _ = thread.join();
        }
    }
}
