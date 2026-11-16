// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! macOS cross-process host evidence.

use std::{
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixDatagram,
    },
    ptr,
    time::Duration,
};

use serde::{Deserialize, Serialize};

/// The subsystem that produced a heartbeat.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostHeartbeatSource {
    Tui,
    Tokio,
}

/// An observation about the watched host process.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostObservation {
    HeartbeatStarted(HostHeartbeatSource),
    HeartbeatMissed(HostHeartbeatSource),
    HeartbeatResumed(HostHeartbeatSource),
    HeartbeatChannelFailed,
    Exited,
}

/// Diagnostic evidence from one kernel exit subscription. PID is for correlation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HostObservationRecord {
    pub host_pid: u32,
    pub observation: HostObservation,
}

/// One-way nonblocking heartbeat producer. The kernel bounds pending datagrams.
#[derive(Debug)]
pub struct HostPulse(UnixDatagram);

impl HostPulse {
    /// Create a private heartbeat channel with a blocking-independent sender.
    pub fn channel() -> io::Result<(Self, UnixDatagram)> {
        let (send, receive) = UnixDatagram::pair()?;
        send.set_nonblocking(true)?;
        receive.set_nonblocking(true)?;
        Ok((Self(send), receive))
    }

    /// Record progress without waiting for the observer or its output sink.
    pub fn pulse(&self, source: HostHeartbeatSource) {
        let byte = match source {
            HostHeartbeatSource::Tui => 1,
            HostHeartbeatSource::Tokio => 2,
        };
        let mut pulse = [byte; 9];
        pulse[1..].copy_from_slice(&monotonic_ns().to_le_bytes());
        let _ = self.0.send(&pulse);
    }
}

/// A process exit subscription and two independently armed heartbeat clocks.
pub struct HostObserver {
    queue: OwnedFd,
    pulses: Option<UnixDatagram>,
    timeout: Duration,
    clocks: [HeartbeatClock; 2],
}

#[derive(Default)]
struct HeartbeatClock {
    last: Option<u64>,
    missed: bool,
}

impl HostObserver {
    /// Subscribe to an existing process. A failed registration yields an error,
    /// without asserting exit or attaching to another PID.
    pub fn attach(pid: u32, pulses: UnixDatagram, timeout: Duration) -> io::Result<Self> {
        // SAFETY: kqueue has no arguments and returns a new owned descriptor.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful kqueue returned a unique descriptor.
        let queue = unsafe { OwnedFd::from_raw_fd(fd) };
        let registration = libc::kevent {
            ident: pid as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ONESHOT,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: ptr::null_mut(),
        };
        // SAFETY: registration is one initialized kevent and there is no output buffer.
        if unsafe { libc::kevent(fd, &registration, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        pulses.set_nonblocking(true)?;
        Ok(Self {
            queue,
            pulses: Some(pulses),
            timeout,
            clocks: Default::default(),
        })
    }

    /// Wait for kernel exit or sample the heartbeat clocks. Reports are delivered
    /// synchronously in the observer process, which is independent of the host.
    pub fn sample(&mut self, report: &mut impl FnMut(HostObservation)) -> io::Result<bool> {
        if self.kernel_exit_observed()? {
            report(HostObservation::Exited);
            return Ok(false);
        }
        self.drain_pulses(report);
        self.report_heartbeat_changes(report);
        Ok(true)
    }

    fn kernel_exit_observed(&self) -> io::Result<bool> {
        let interval =
            (self.timeout / 4).clamp(Duration::from_millis(1), Duration::from_millis(100));
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: interval.subsec_nanos().into(),
        };
        let mut event = std::mem::MaybeUninit::<libc::kevent>::uninit();
        // SAFETY: one output kevent fits the supplied buffer. The timeout is initialized.
        let count = unsafe {
            libc::kevent(
                self.queue.as_raw_fd(),
                ptr::null(),
                0,
                event.as_mut_ptr(),
                1,
                &timeout,
            )
        };
        if count < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Interrupted {
                Ok(false)
            } else {
                Err(error)
            };
        }
        if count == 1 {
            // SAFETY: kevent reported one initialized event.
            let event = unsafe { event.assume_init() };
            if event.flags & libc::EV_ERROR != 0 {
                return Err(io::Error::from_raw_os_error(event.data as i32));
            }
            if event.fflags & libc::NOTE_EXIT != 0 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn drain_pulses(&mut self, report: &mut impl FnMut(HostObservation)) {
        if let Some(pulses) = &self.pulses {
            for _ in 0..256 {
                let mut pulse = [0; 10];
                match pulses.recv(&mut pulse) {
                    Ok(9) if matches!(pulse[0], 1 | 2) => {
                        let index = (pulse[0] - 1) as usize;
                        let clock = &mut self.clocks[index];
                        if clock.last.is_none() {
                            report(HostObservation::HeartbeatStarted(
                                [HostHeartbeatSource::Tui, HostHeartbeatSource::Tokio][index],
                            ));
                        }
                        let timestamp = u64::from_le_bytes(pulse[1..9].try_into().unwrap());
                        clock.last = Some(clock.last.unwrap_or(0).max(timestamp));
                    }
                    Ok(_) => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.pulses = None;
                        report(HostObservation::HeartbeatChannelFailed);
                        break;
                    }
                }
            }
        }
    }

    fn report_heartbeat_changes(&mut self, report: &mut impl FnMut(HostObservation)) {
        let now = monotonic_ns();
        for (source, clock) in [HostHeartbeatSource::Tui, HostHeartbeatSource::Tokio]
            .into_iter()
            .zip(&mut self.clocks)
        {
            if let Some(last) = clock.last {
                let missed = Duration::from_nanos(now.saturating_sub(last)) >= self.timeout;
                if missed != clock.missed {
                    clock.missed = missed;
                    report(if missed {
                        HostObservation::HeartbeatMissed(source)
                    } else {
                        HostObservation::HeartbeatResumed(source)
                    });
                }
            }
        }
    }
}

unsafe extern "C" {
    fn clock_gettime_nsec_np(clock_id: libc::clockid_t) -> u64;
}

fn monotonic_ns() -> u64 {
    // SAFETY: Darwin exposes this system-wide monotonic clock to all processes.
    unsafe { clock_gettime_nsec_np(libc::CLOCK_MONOTONIC_RAW) }
}
