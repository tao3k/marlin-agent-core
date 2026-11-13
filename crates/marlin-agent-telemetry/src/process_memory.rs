// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Local macOS physical footprint sampling for Runtime admission and diagnostics.

use std::io;

use marlin_agent_protocol::{ProcessMemorySample, ProcessMemorySampler};

/// Sample macOS physical footprint from the current host and direct child PIDs.
#[derive(Debug)]
pub struct MacOsProcessMemorySampler;

impl ProcessMemorySampler for MacOsProcessMemorySampler {
    fn sample(&self, direct_child_pids: &[u32]) -> io::Result<ProcessMemorySample> {
        let host_bytes = physical_footprint(std::process::id())?;
        let mut child_bytes = 0_u64;
        for &pid in direct_child_pids {
            match physical_footprint(pid) {
                Ok(bytes) => child_bytes = child_bytes.saturating_add(bytes),
                Err(error) if error.raw_os_error() == Some(libc::ESRCH) => (),
                Err(error) => return Err(error),
            }
        }
        Ok(ProcessMemorySample {
            host_bytes,
            child_bytes,
        })
    }
}

fn physical_footprint(pid: u32) -> io::Result<u64> {
    let pid = i32::try_from(pid).map_err(io::Error::other)?;
    let mut info = std::mem::MaybeUninit::<libc::rusage_info_v0>::uninit();
    // SAFETY: the buffer is correctly sized for RUSAGE_INFO_V0 and borrowed for the syscall.
    if unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V0, info.as_mut_ptr().cast()) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful proc_pid_rusage initialized the V0 structure.
    Ok(unsafe { info.assume_init() }.ri_phys_footprint)
}
