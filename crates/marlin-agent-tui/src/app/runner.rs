// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal event loop and process entry.

use super::{
    AgentId, App, Arc, AtomicU64, BracketedPaste, CliAction, FrameSchedule, HistorySearch,
    INPUT_QUEUE_ITEMS, InputDecoder, Instant, JournalGateway, Key, MouseMotion, Ordering,
    ProjectId, QueuedReply, REPLY_BATCH_LIMIT, REPLY_QUEUE_BYTES, REPLY_QUEUE_ITEMS,
    RuntimeEdgePolicy, RuntimeStreamGateway, ScrollAnchor, SessionBrowser, SessionId,
    SharedGateway, SignalMonitor, Terminal, TerminalInput, TermionBackend, TranscriptViewportCache,
    TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode,
    TursoOptimizationProfile, UiBrowseDirection, UiHost, UiKeyAction, UiOpenFacts, config, draw,
    io, list_sessions, load_session, mouse_over_minibuffer, mpsc, parse_cli, reply_channel,
    safe_text, thread,
};
use std::{
    env,
    io::{IsTerminal, Write},
    path::PathBuf,
};
use termion::{input::TermReadEventsAndRaw, raw::IntoRawMode, screen::IntoAlternateScreen};

pub(crate) fn process_replies(
    app: &mut App,
    receiver: &mut tokio::sync::mpsc::Receiver<QueuedReply>,
    limit: usize,
) -> usize {
    let mut processed = 0;
    while processed < limit {
        let Ok(event) = receiver.try_recv() else {
            break;
        };
        let (id, reply) = event.into_parts();
        app.receive(id, reply);
        processed += 1;
    }
    processed
}

fn run(
    path: PathBuf,
    selected_session: Option<SessionId>,
) -> Result<Option<i32>, Box<dyn std::error::Error>> {
    let startup = Instant::now();
    let trace_startup = |stage: &str| {
        if env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
            eprintln!("marlin-tui startup {stage}: {:?}", startup.elapsed());
        }
    };
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("marlin-tui requires a terminal on stdin and stdout".into());
    }
    let config = config::Config::load(&path)?;
    if selected_session.is_some() && config.session.is_none() {
        return Err("--session requires a [session] storage configuration".into());
    }
    let (endpoint, catalog) = config.catalog()?;
    trace_startup("configuration");
    // Package resolution uses SDK-owned process environment during startup;
    // perform it before starting Tokio workers or the terminal input thread.
    let _ = writeln!(io::stderr(), "Starting interface…");
    #[cfg(feature = "linked-ui")]
    let scheme_runtime = super::super::linked_ui::initialize()?;
    #[cfg(not(feature = "linked-ui"))]
    let scheme_runtime = gerbil_scheme::GerbilRuntime::initialize()?;
    trace_startup("Scheme VM");
    #[cfg(not(feature = "linked-ui"))]
    scheme_runtime
        .set_package_resolution_progress(gerbil_scheme::PackageResolutionProgress::Quiet)?;
    #[cfg(not(feature = "linked-ui"))]
    let directory = env::current_dir()?;
    #[cfg(not(feature = "linked-ui"))]
    let directory = directory
        .to_str()
        .ok_or("UI package directory is not UTF-8")?;
    let model_label = format!("{}/{}", endpoint.provider.as_str(), endpoint.model.as_str());
    let session_label = selected_session
        .as_ref()
        .map(SessionId::as_str)
        .or_else(|| {
            config
                .session
                .as_ref()
                .map(|session| session.session.as_str())
        })
        .unwrap_or("temporary");
    let ui_facts = UiOpenFacts {
        model_label: &model_label,
        session_label,
        status: &[],
        extensions: &[],
        method_bundles: &[],
    };
    #[cfg(feature = "linked-ui")]
    // SAFETY: the binary links these exports from the archive built from the
    // same selected Scheme programme as `linked_ui::initialize`.
    let mut ui = unsafe {
        UiHost::open_linked(
            &scheme_runtime,
            super::super::linked_ui::exports(),
            &ui_facts,
        )?
    };
    #[cfg(not(feature = "linked-ui"))]
    let mut ui = UiHost::open(&scheme_runtime, directory, &ui_facts)?;
    trace_startup("Scheme UI publication");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    trace_startup("Tokio runtime");
    let mut app = App::new(endpoint);
    app.set_models(catalog.clone());
    let open_timeout_ms = config
        .timeout_seconds
        .checked_mul(1000)
        .ok_or("stream-open timeout is too large")?;
    let gateway = RuntimeStreamGateway::new(
        catalog,
        RuntimeEdgePolicy::new().with_timeout_ms(open_timeout_ms),
    )?;
    trace_startup("model gateway");
    app.set_runtime_edge(gateway.receipt().clone());
    let base_gateway = gateway.clone();
    let mut gateway = if let Some(session) = config.session {
        let session_id = match selected_session {
            Some(id) => id,
            None => SessionId::new(session.session)?,
        };
        let (journal, turns, browser) = runtime.block_on(async {
            let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
                path: session.path,
                optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
                batch_transaction_mode: TursoBatchTransactionMode::Immediate,
            })
            .await?;
            let project = ProjectId::new(session.project)?;
            let agent = AgentId::new(session.agent)?;
            let browser = SessionBrowser::new(storage.clone(), project.clone(), agent.clone());
            let (journal, turns) =
                load_session(storage, project, session_id.clone(), agent).await?;
            Ok::<_, Box<dyn std::error::Error>>((journal, turns, browser))
        })?;
        app.session_browser = Some(browser);
        app.current_session = Some(session_id.clone());
        app.transcript
            .push(("Session".into(), session_id.as_str().to_owned()));
        app.restore(turns);
        SharedGateway::new(JournalGateway::new(gateway, journal))
    } else {
        SharedGateway::new(gateway)
    };
    let signals = SignalMonitor::new(&runtime)?;
    trace_startup("session and signals");
    let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let stdout = BracketedPaste::enable(MouseMotion::enable(stdout)?)?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let (keys_tx, keys_rx) = mpsc::sync_channel(INPUT_QUEUE_ITEMS);
    let mouse_position = Arc::new(AtomicU64::new(0));
    let input_mouse_position = Arc::clone(&mouse_position);
    thread::spawn(move || {
        let mut decoder = InputDecoder::default();
        for event in io::stdin().events_and_raw() {
            match event {
                Ok((event, raw)) => {
                    if let Some(input) = decoder.accept(event, &raw) {
                        match input {
                            TerminalInput::MouseMove(x, y) => {
                                input_mouse_position
                                    .store((u64::from(y) << 16) | u64::from(x), Ordering::Relaxed);
                            }
                            input => {
                                if keys_tx.send(Ok(input)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    let _ = keys_tx.send(Err(error));
                    break;
                }
            }
        }
    });
    let (reply_tx, mut reply_rx) = reply_channel(REPLY_QUEUE_ITEMS, REPLY_QUEUE_BYTES);
    let mut schedule = FrameSchedule::new();
    let mut viewport_cache = TranscriptViewportCache::default();
    let mut scroll_anchor = ScrollAnchor::default();
    let mut last_size = None;
    let mut first_frame_elapsed = None;
    let mut failed_ui_labels: Option<(String, String)> = None;
    let mut signal_exit = None;
    let result = (|| -> io::Result<()> {
        loop {
            if app.command_selection.as_ref().is_some_and(|(query, _)| {
                query != app.composer.text() || !app.composer.cursor_at_end()
            }) {
                app.command_selection = None;
            }
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            let processed = process_replies(&mut app, &mut reply_rx, REPLY_BATCH_LIMIT);
            let sessions_changed = app.poll_sessions();
            let workspace_changed = if let Some(result) = app.workspace.poll() {
                match result {
                    Ok(view) => app.workspace.show(view),
                    Err(error) => app.transcript.push((
                        "Workspace".into(),
                        format!("Workspace read failed: {error}"),
                    )),
                }
                true
            } else {
                false
            };
            let switched = app.poll_session_switch(&mut gateway, &base_gateway);
            let search_finished = app
                .history_search
                .as_mut()
                .is_some_and(|search| search.step(&app.history));
            if processed > 0 || sessions_changed || switched || workspace_changed || search_finished
            {
                schedule.mark_dirty();
            }
            let size = termion::terminal_size()?;
            if last_size != Some(size) {
                last_size = Some(size);
                schedule.mark_dirty();
            }
            let mouse = mouse_position.load(Ordering::Relaxed);
            let hovered = mouse_over_minibuffer(
                &app,
                &ui.view().layout,
                size.0,
                size.1,
                (mouse >> 16) as u16,
            ) && (mouse as u16) <= size.0;
            if hovered != app.minibuffer_hovered {
                app.minibuffer_hovered = hovered;
                schedule.mark_dirty();
            }
            let now = Instant::now();
            if schedule.should_draw(now) {
                let endpoint = app.endpoint();
                let model = format!("{}/{}", endpoint.provider.as_str(), endpoint.model.as_str());
                let session = app.session_id().map_or("temporary", |id| id.as_str());
                if ui.view().model_label == model && ui.view().session_label == session {
                    failed_ui_labels = None;
                }
                if (ui.view().model_label != model || ui.view().session_label != session)
                    && !failed_ui_labels
                        .as_ref()
                        .is_some_and(|(old_model, old_session)| {
                            old_model == &model && old_session == session
                        })
                {
                    if let Err(error) = ui.refresh_labels(&model, session) {
                        let attempted = (model, session.to_owned());
                        app.transcript.push((
                            "UI".into(),
                            safe_text(&format!("UI label refresh failed: {error}")),
                        ));
                        failed_ui_labels = Some(attempted);
                    } else {
                        failed_ui_labels = None;
                    }
                }
                let mut effective_scroll = app.scroll_up;
                terminal.draw(|frame| {
                    effective_scroll = draw(
                        frame,
                        &app,
                        ui.view(),
                        &mut viewport_cache,
                        &mut scroll_anchor,
                    )
                })?;
                app.set_effective_scroll(effective_scroll);
                schedule.drawn(Instant::now());
                if first_frame_elapsed.is_none() {
                    first_frame_elapsed = Some(startup.elapsed());
                }
            }
            let wait = if processed == REPLY_BATCH_LIMIT
                || app
                    .history_search
                    .as_ref()
                    .is_some_and(HistorySearch::is_searching)
            {
                std::time::Duration::ZERO
            } else {
                schedule.wait_duration(Instant::now())
            };
            let received = keys_rx.recv_timeout(wait);
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            if matches!(&received, Ok(Ok(_))) {
                schedule.mark_dirty();
            }
            if matches!(&received, Ok(Ok(input)) if !matches!(input, TerminalInput::Key(Key::Ctrl('c'))))
            {
                app.disarm_exit();
            }
            if let Some(search) = app.history_search.as_mut() {
                match received {
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => break,
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('c') | Key::Esc))) => {
                        app.history_search = None;
                        app.disarm_exit();
                    }
                    Ok(Ok(TerminalInput::Key(Key::Char('\n')))) => app.accept_history_search(),
                    Ok(Ok(TerminalInput::Key(Key::Ctrl('r')))) => search.previous(&app.history),
                    Ok(Ok(TerminalInput::Key(Key::Backspace))) => search.backspace(&app.history),
                    Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() => {
                        search.insert(&c.to_string(), &app.history)
                    }
                    Ok(Ok(TerminalInput::Paste(text))) => search.insert(&text, &app.history),
                    Ok(Err(e)) => return Err(e),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    _ => {}
                }
                continue;
            }
            if let Ok(Ok(TerminalInput::Key(key))) = &received {
                let menu_active = app.composer.cursor_at_end()
                    && ui.accepts_text(app.command_draft.is_some())
                    && ui.has_command_prefix(app.composer.text());
                if menu_active && matches!(key, Key::Up | Key::Down) {
                    let direction = if *key == Key::Up {
                        UiBrowseDirection::Previous
                    } else {
                        UiBrowseDirection::Next
                    };
                    match ui.browse_command(app.composer.text(), app.selected_command(), direction)
                    {
                        Ok(choice) => {
                            app.command_selection =
                                choice.map(|name| (app.composer.text().to_owned(), name))
                        }
                        Err(error) => app.transcript.push((
                            "UI".into(),
                            safe_text(&format!("Command menu failed: {error}")),
                        )),
                    }
                    continue;
                }
                if *key == Key::Char('\t') && menu_active {
                    let completed = app
                        .selected_command()
                        .and_then(|name| ui.complete_selected_command(app.composer.text(), name))
                        .or_else(|| ui.complete_command(app.composer.text()));
                    if let Some(completed) = completed {
                        app.composer.take();
                        app.composer.insert_text(&completed);
                    }
                    app.command_selection = None;
                    continue;
                }
                if *key == Key::Char('\n')
                    && menu_active
                    && let Some(completed) = app
                        .selected_command()
                        .and_then(|name| ui.complete_selected_command(app.composer.text(), name))
                {
                    if app.accept_command_completion(&completed) {
                        app.submit(&gateway, &runtime, &reply_tx);
                    }
                    continue;
                }
                match ui.route_key(
                    *key,
                    app.command_draft.is_some(),
                    app.composer.text().trim_start().starts_with('/'),
                ) {
                    Ok(UiKeyAction::Handled) => continue,
                    Ok(UiKeyAction::Submit) => {
                        app.submit(&gateway, &runtime, &reply_tx);
                        continue;
                    }
                    Ok(UiKeyAction::OpenCommands) => {
                        app.toggle_command();
                        continue;
                    }
                    Ok(UiKeyAction::Scroll(up)) => {
                        app.scroll(up);
                        continue;
                    }
                    Ok(UiKeyAction::Unhandled) => {}
                    Err(error) => {
                        app.transcript
                            .push(("UI".into(), safe_text(&format!("UI event failed: {error}"))));
                        continue;
                    }
                }
            }
            let accepts_text = ui.accepts_text(app.command_draft.is_some());
            match received {
                Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) if app.interrupt() => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) => {}
                Ok(Ok(TerminalInput::Key(Key::Ctrl('o')))) => app.toggle_command(),
                Ok(Ok(TerminalInput::Key(Key::Ctrl('r')))) => app.start_history_search(),
                Ok(Ok(TerminalInput::Key(Key::Alt('p')))) if accepts_text => {
                    app.navigate_prompts(true)
                }
                Ok(Ok(TerminalInput::Key(Key::Alt('n')))) if accepts_text => {
                    app.navigate_prompts(false)
                }
                Ok(Ok(TerminalInput::Key(Key::Esc))) => app.cancel(),
                Ok(Ok(TerminalInput::Key(Key::Ctrl('n')))) if accepts_text => {
                    app.composer.insert('\n')
                }
                Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() && accepts_text => {
                    app.composer.insert(c)
                }
                Ok(Ok(TerminalInput::Key(Key::Left))) if accepts_text => app.composer.left(),
                Ok(Ok(TerminalInput::Key(Key::Right))) if accepts_text => app.composer.right(),
                Ok(Ok(TerminalInput::Key(Key::Home | Key::Ctrl('a')))) if accepts_text => {
                    app.composer.line_start()
                }
                Ok(Ok(TerminalInput::Key(Key::End | Key::Ctrl('e')))) if accepts_text => {
                    app.composer.line_end()
                }
                Ok(Ok(TerminalInput::Key(Key::Up))) if accepts_text => {
                    app.composer.move_vertical(size.0.saturating_sub(2), false)
                }
                Ok(Ok(TerminalInput::Key(Key::Down))) if accepts_text => {
                    app.composer.move_vertical(size.0.saturating_sub(2), true)
                }
                Ok(Ok(TerminalInput::Key(Key::Backspace))) if accepts_text => {
                    app.composer.backspace()
                }
                Ok(Ok(TerminalInput::Key(Key::Delete))) if accepts_text => app.composer.delete(),
                Ok(Ok(TerminalInput::Key(Key::PageUp))) => app.scroll(true),
                Ok(Ok(TerminalInput::Key(Key::PageDown))) => app.scroll(false),
                Ok(Ok(TerminalInput::Paste(text))) if accepts_text => {
                    app.composer.insert_text(&text)
                }
                Ok(Ok(TerminalInput::PasteRejected)) => app.transcript.push((
                    "Error".into(),
                    "Paste rejected: invalid UTF-8 or larger than 1 MiB.".into(),
                )),
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                _ => {}
            }
        }
        Ok(())
    })();
    trace_startup("event loop exit");
    app.cancel();
    terminal.show_cursor()?;
    drop(terminal); // alternate-screen and raw-mode guards restore the terminal
    trace_startup("terminal restored");
    if env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some()
        && let Some(elapsed) = first_frame_elapsed
    {
        eprintln!("marlin-tui startup first frame: {elapsed:?}");
    }
    result?;
    drop(gateway);
    trace_startup("gateway dropped");
    drop(runtime);
    trace_startup("Tokio runtime dropped");
    drop(ui);
    trace_startup("UI session dropped");
    drop(scheme_runtime);
    trace_startup("Scheme VM dropped");
    Ok(signal_exit)
}

/// Start the terminal client with the process argument list.
pub fn run_cli() {
    let args = match parse_cli(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let result = match args.action {
        CliAction::Open { session } => run(args.config, session),
        CliAction::List { after } => list_sessions(&args.config, after).map(|()| None),
    };
    match result {
        Ok(Some(code)) => std::process::exit(code),
        Ok(None) => {}
        Err(error) => {
            let _ = writeln!(io::stderr(), "marlin-tui: {error}");
            std::process::exit(1);
        }
    }
}
