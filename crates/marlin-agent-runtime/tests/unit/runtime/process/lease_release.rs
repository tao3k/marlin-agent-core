// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{LeasedPipe, ProcessLease, ProcessPermit};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Semaphore;

#[tokio::test]
async fn confirmed_process_does_not_release_until_pipe_endpoint_is_dropped() {
    struct EndpointProbe {
        quota: Arc<Semaphore>,
        closed_before_release: Arc<AtomicBool>,
    }
    impl Drop for EndpointProbe {
        fn drop(&mut self) {
            self.closed_before_release
                .store(self.quota.available_permits() == 0, Ordering::SeqCst);
        }
    }
    let quota = Arc::new(Semaphore::new(1));
    let permit = quota.clone().acquire_owned().await.unwrap();
    let mut process = ProcessPermit(Some(Arc::new(ProcessLease {
        permit: Some(permit),
        confirmed: AtomicBool::new(false),
        ownership: None,
    })));
    let observed = Arc::new(AtomicBool::new(false));
    let pipe = LeasedPipe {
        inner: EndpointProbe {
            quota: quota.clone(),
            closed_before_release: observed.clone(),
        },
        _lease: process.0.clone(),
    };
    process.release();
    assert_eq!(quota.available_permits(), 0);
    drop(pipe);
    assert!(observed.load(Ordering::SeqCst));
    assert_eq!(quota.available_permits(), 1);
}
