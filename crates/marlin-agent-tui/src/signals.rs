// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Process-signal observation for orderly terminal shutdown.

#[cfg(unix)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

#[cfg(unix)]
pub(crate) struct SignalMonitor {
    exit_code: Arc<AtomicI32>,
    suspend_requested: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

#[cfg(unix)]
impl SignalMonitor {
    /// Register before entering raw mode, so failure cannot strand terminal modes.
    pub(crate) fn new(runtime: &tokio::runtime::Runtime) -> std::io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        let interrupt_kind = SignalKind::interrupt();
        let terminate_kind = SignalKind::terminate();
        let hangup_kind = SignalKind::hangup();
        let suspend_kind = SignalKind::from_raw(libc::SIGTSTP);
        let (mut interrupt, mut terminate, mut hangup, mut suspend) = {
            let _entered = runtime.enter();
            (
                signal(interrupt_kind)?,
                signal(terminate_kind)?,
                signal(hangup_kind)?,
                signal(suspend_kind)?,
            )
        };
        let exit_code = Arc::new(AtomicI32::new(0));
        let observed = exit_code.clone();
        let suspend_requested = Arc::new(AtomicBool::new(false));
        let requested = suspend_requested.clone();
        let task = runtime.spawn(async move {
            loop {
                let number = tokio::select! {
                    _ = interrupt.recv() => Some(interrupt_kind.as_raw_value()),
                    _ = terminate.recv() => Some(terminate_kind.as_raw_value()),
                    _ = hangup.recv() => Some(hangup_kind.as_raw_value()),
                    _ = suspend.recv() => {
                        requested.store(true, Ordering::Release);
                        None
                    },
                };
                if let Some(number) = number {
                    observed.store(128 + number, Ordering::Release);
                    break;
                }
            }
        });
        Ok(Self {
            exit_code,
            suspend_requested,
            task,
        })
    }

    pub(crate) fn observed_exit_code(&self) -> Option<i32> {
        match self.exit_code.load(Ordering::Acquire) {
            0 => None,
            code => Some(code),
        }
    }

    pub(crate) fn take_suspend_request(&self) -> bool {
        self.suspend_requested.swap(false, Ordering::AcqRel)
    }
}

#[cfg(unix)]
impl Drop for SignalMonitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(not(unix))]
pub(crate) struct SignalMonitor;

#[cfg(not(unix))]
impl SignalMonitor {
    pub(crate) fn new(_: &tokio::runtime::Runtime) -> std::io::Result<Self> {
        Ok(Self)
    }
    pub(crate) fn observed_exit_code(&self) -> Option<i32> {
        None
    }
}
