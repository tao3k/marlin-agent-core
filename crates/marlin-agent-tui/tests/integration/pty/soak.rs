// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{StopOnDrop, Terminal, TerminalScreen, accept_until, read_request};
use std::{
    fs,
    io::Write,
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "macos")]
fn sample(pid: u32) -> (u64, i32, usize) {
    let pid = i32::try_from(pid).unwrap();
    let mut task = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_taskinfo>();
    // SAFETY: the kernel writes at most the supplied size into a valid buffer.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTASKINFO,
            0,
            task.as_mut_ptr().cast(),
            size as i32,
        )
    };
    assert_eq!(
        written,
        size as i32,
        "proc_pidinfo task: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: the full structure was initialized by proc_pidinfo above.
    let task = unsafe { task.assume_init() };
    // SAFETY: a null buffer asks libproc for the required byte count.
    let needed =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    assert!(
        needed > 0,
        "proc_pidinfo fd size: {}",
        std::io::Error::last_os_error()
    );
    let unit = std::mem::size_of::<libc::proc_fdinfo>();
    let capacity = needed as usize + 32 * unit;
    let mut buffer = vec![0_u8; capacity];
    // SAFETY: buffer is valid for exactly capacity writable bytes.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            buffer.as_mut_ptr().cast(),
            capacity as i32,
        )
    };
    assert!(
        written >= 0 && (written as usize) < capacity && (written as usize).is_multiple_of(unit),
        "incomplete proc_pidinfo fd sample"
    );
    (
        task.pti_resident_size,
        task.pti_threadnum,
        written as usize / unit,
    )
}

// A real linked TUI and its Tokio runtime, with a deterministic local streaming
// provider. Resource samples are evidence, not a universal leak threshold.
#[test]
#[ignore = "macOS PTY soak with loopback provider; requires linked Scheme package"]
#[cfg(target_os = "macos")]
fn repeated_stream_cancellation_soak() {
    let seconds = std::env::var("MARLIN_TUI_SOAK_SECONDS")
        .ok()
        .map(|s| s.parse::<u64>().expect("soak seconds"))
        .unwrap_or(120);
    assert!(seconds > 0);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let _stop_guard = StopOnDrop(stop.clone());
    let server_stop = stop.clone();
    let (connected_tx, connected_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut iteration = 0;
        while let Some(mut stream) = accept_until(&listener, &server_stop) {
            let _ = read_request(&mut stream);
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            if stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").is_err() { continue; }
            iteration += 1;
            if connected_tx.send(iteration).is_err() {
                break;
            }
            let event = format!(
                "data: {{\"type\":\"response.output_text.delta\",\"delta\":\"SOAK{iteration} \"}}\n\n"
            );
            while !server_stop.load(Ordering::Relaxed) {
                if stream.write_all(event.as_bytes()).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(&config, format!("provider=\"openai\"\nmodel=\"gpt-fixture\"\nendpoint=\"http://{address}/v1/responses\"\nprotocol=\"open-ai-responses\"\ncredential_env=\"MARLIN_TUI_FIXTURE_KEY\"\n")).unwrap();
    let metrics_path = directory.path().join("runtime-metrics.csv");
    let variables = [(
        "MARLIN_TUI_RUNTIME_METRICS_FILE",
        metrics_path.to_string_lossy().into_owned(),
    )];
    let mut terminal = Terminal::start_with_env(&config, &[], &variables);
    terminal.wait_status("Message: Enter send");
    thread::sleep(Duration::from_millis(600));
    let baseline = fs::read_to_string(&metrics_path).expect("initial runtime metrics");
    let baseline: Vec<usize> = baseline
        .lines()
        .last()
        .expect("initial runtime sample")
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(baseline.len(), 4);
    let startup_output = terminal.output.clone();
    let mut screen = TerminalScreen::new();
    screen.update(&startup_output);
    terminal.output.clear();
    let mut parsed = 0;
    let pid = terminal.child.id();
    let started = Instant::now();
    let mut cycles = 0;
    let mut max_cancel = Duration::ZERO;
    let mut max_connect = Duration::ZERO;
    let mut max_render = Duration::ZERO;
    let mut max_cycle = Duration::ZERO;
    let mut max_sampling = Duration::ZERO;
    while started.elapsed() < Duration::from_secs(seconds) {
        let cycle_start = Instant::now();
        cycles += 1;
        terminal.send(b"soak\r");
        assert_eq!(
            connected_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("provider connection"),
            cycles
        );
        max_connect = max_connect.max(cycle_start.elapsed());
        let render_started = Instant::now();
        let marker = format!("SOAK{cycles}");
        terminal.wait_for(Duration::from_secs(10), "fresh stream output", |output| {
            screen.update(&output[parsed..]);
            parsed = output.len();
            screen.line(28).contains("Working")
                && (0..28).any(|row| {
                    screen
                        .line(row)
                        .split_ascii_whitespace()
                        .any(|word| word == marker)
                })
        });
        max_render = max_render.max(render_started.elapsed());
        let cancel = Instant::now();
        terminal.send(b"\x03");
        terminal.wait_for(Duration::from_secs(10), "fresh cancellation", |output| {
            screen.update(&output[parsed..]);
            parsed = output.len();
            screen.line(28).contains("Ready")
                && screen.line(29).contains("Request cancelled locally")
        });
        max_cancel = max_cancel.max(cancel.elapsed());
        if cycles == 1 || cycles % 10 == 0 {
            let sampling_started = Instant::now();
            let (rss, threads, fds) = sample(pid);
            max_sampling = max_sampling.max(sampling_started.elapsed());
            eprintln!(
                "soak cycle={cycles} elapsed={:?} rss_bytes={rss} open_fds={fds} threads={threads} max_cancel={max_cancel:?} sampling={:?}",
                started.elapsed(),
                sampling_started.elapsed()
            );
        }
        // Keep the raw transcript bounded while preserving the screen state.
        terminal.output.clear();
        parsed = 0;
        thread::sleep(Duration::from_millis(100));
        max_cycle = max_cycle.max(cycle_start.elapsed());
    }
    stop.store(true, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(600));
    let metrics = fs::read_to_string(&metrics_path).expect("runtime metrics");
    let samples: Vec<Vec<usize>> = metrics
        .lines()
        .map(|line| {
            line.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        })
        .collect();
    assert!(samples.len() >= 2, "insufficient runtime task samples");
    let first = &samples[0];
    let last = samples.last().unwrap();
    assert_eq!(last.len(), 4);
    let peak = samples.iter().map(|sample| sample[0]).max().unwrap();
    eprintln!(
        "tokio samples={} first={first:?} last={last:?} peak_tasks={peak}",
        samples.len()
    );
    assert!(
        last[0] <= baseline[0] + 1,
        "idle Tokio tasks grew from {} to {}",
        baseline[0],
        last[0]
    );
    assert!(
        last[2] <= baseline[2],
        "tracked tasks did not return to baseline"
    );
    assert_eq!(last[3], 0, "managed processes remain active");
    terminal.send(b"\x11");
    terminal.wait_exit(Duration::from_secs(10));
    terminal.output.splice(0..0, startup_output);
    terminal.assert_restored();
    server.join().unwrap();
    eprintln!(
        "soak complete: cycles={cycles} elapsed={:?} max_connect={max_connect:?} max_render={max_render:?} max_cancel={max_cancel:?} max_sampling={max_sampling:?} max_cycle={max_cycle:?}",
        started.elapsed()
    );
}
