// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Optional task and process observer outside the terminal event loop.

// Optional diagnostic observer. It runs outside Tokio so it does not inflate
// the runtime task count and never performs file I/O on the UI thread.
pub(super) struct RuntimeProbe(Option<(std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>)>);

impl RuntimeProbe {
    pub(super) fn start(
        runtime: &tokio::runtime::Runtime,
        owner: &marlin_agent_runtime::TokioAgentRuntime,
    ) -> Self {
        let Some(path) = std::env::var_os("MARLIN_TUI_RUNTIME_METRICS_FILE") else {
            return Self(None);
        };
        let metrics = runtime.handle().metrics();
        let tracker = owner.task_tracker();
        let processes = owner.process_registry();
        let (stop, receiver) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            use std::io::Write;
            let Ok(mut file) = std::fs::File::create(path) else {
                return;
            };
            let mut stopped = false;
            loop {
                if writeln!(
                    file,
                    "{},{},{},{}",
                    metrics.num_alive_tasks(),
                    metrics.num_workers(),
                    tracker.active_task_count(),
                    processes.active_processes().len()
                )
                .is_err()
                    || stopped
                {
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
