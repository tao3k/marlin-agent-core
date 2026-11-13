// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal event loop and process entry.

use super::{
    AgentId, App, Arc, AtomicU64, CliAction, FrameSchedule, HistorySearch, INPUT_QUEUE_ITEMS,
    InputDecoder, Instant, JournalGateway, Key, MessageEvent, MessageSeverity, MessageSource,
    Ordering, ProjectId, QueuedReply, REPLY_BATCH_LIMIT, REPLY_QUEUE_BYTES, REPLY_QUEUE_ITEMS,
    RUNTIME_EVENT_BATCH_LIMIT, RuntimeEdgePolicy, RuntimeStreamGateway, ScrollAnchor,
    SessionBrowser, SessionId, SharedGateway, SignalMonitor, TerminalInput, TerminalOwner,
    TranscriptViewportCache, TursoAgentStorage, TursoAgentStorageConfig, TursoBatchTransactionMode,
    TursoOptimizationProfile, UiBrowseDirection, UiHost, UiKeyAction, UiOpenFacts,
    WheelAccumulator, config, draw, frame_regions, io, list_sessions, load_session,
    mouse_over_minibuffer, mpsc, parse_cli, reply_channel, thread,
};
use marlin_agent_telemetry::{OpenTelemetry, TelemetryServiceName, TraceTarget};
use marlin_agent_ui::UiEditorAction;
use std::{
    env,
    io::{IsTerminal, Write},
    path::PathBuf,
};

use super::runtime_probe::RuntimeProbe;
use super::tool_review::{open_tool_review, poll_tool_review, submit_tool_review};
use super::ui_events::{apply_ui_request, browse_menu, command_menu_active};
use termion::input::TermReadEventsAndRaw;

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

pub(super) fn publish_error(
    app: &mut App,
    source: MessageSource,
    summary: &str,
    error: &impl std::fmt::Display,
) {
    let detail = error.to_string();
    app.publish_message(MessageEvent {
        source,
        severity: MessageSeverity::Error,
        summary,
        detail: &detail,
    });
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
    #[cfg(not(target_os = "macos"))]
    if config.process_budget.is_some() {
        return Err("process memory sampling is unavailable on this host".into());
    }
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
    let mut status = Vec::new();
    if config.process_budget.is_some() {
        status.push(marlin_agent_ui::UiStatusItem::host(
            marlin_agent_ui::UiComponentId::new("runtime/process-budget")?,
            "Managed process budget enabled",
        ));
    }
    let ui_facts = UiOpenFacts {
        model_label: &model_label,
        session_label,
        status: &status,
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
    if let Some(status) = status.first()
        && !ui.view().regions.iter().any(|region| {
            matches!(region, marlin_agent_ui::UiRegion::Status(items) if items.iter().any(|item| item == status))
        })
    {
        return Err("Scheme projection lost the Runtime budget status".into());
    }
    trace_startup("UI session projection");
    #[cfg(feature = "linked-ui")]
    let tool_catalog = if config.session.is_some() {
        let declarations = super::super::linked_ui::tooling_declarations(&scheme_runtime)?;
        Some(Arc::new(
            marlin_agent_tooling::catalog_from_declarations(declarations, env::current_dir()?)
                .map_err(|error| format!("cannot bind Scheme tool catalog: {error:?}"))?,
        ))
    } else {
        None
    };
    #[cfg(not(feature = "linked-ui"))]
    let tool_catalog: Option<Arc<marlin_agent_runtime::ModelToolCatalog>> = None;
    trace_startup("tool catalog binding");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    trace_startup("Tokio runtime");
    #[cfg(target_os = "macos")]
    let mut app = if let Some(budget) = config.process_budget {
        use marlin_agent_runtime::{TokioAgentRuntime, TokioAgentRuntimeBuildRequest};
        let (owner, events) = TokioAgentRuntime::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(64).with_process_budget(
                budget,
                Arc::new(marlin_agent_telemetry::MacOsProcessMemorySampler),
            ),
        );
        App::with_runtime(endpoint, owner, events)
    } else {
        App::new(endpoint)
    };
    #[cfg(not(target_os = "macos"))]
    let mut app = App::new(endpoint);
    app.transcript_policy = Some(ui.view().layout.transcript_policy.clone());
    app.tool_offer_options = tool_catalog
        .as_ref()
        .map(|catalog| catalog.offer().completion_options());
    app.tool_catalog = tool_catalog.clone();
    let runtime_probe = RuntimeProbe::start(&runtime, &app.runtime_owner);
    #[cfg(target_os = "macos")]
    let host_pulse = match env::var_os("MARLIN_TUI_HOST_OBSERVATIONS_FILE")
        .map(|path| marlin_agent_telemetry::start_host_observer(std::path::Path::new(&path)))
        .transpose()
    {
        Ok(pulse) => pulse.map(Arc::new),
        Err(error) => {
            eprintln!("marlin-tui: host observation unavailable: {error}");
            None
        }
    };
    #[cfg(target_os = "macos")]
    let heartbeat_task = host_pulse.as_ref().map(|pulse| {
        let pulse = Arc::clone(pulse);
        app.runtime_owner.spawn_on(&runtime, async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                pulse.pulse(marlin_agent_telemetry::host_watchdog::HostHeartbeatSource::Tokio);
            }
        })
    });
    app.sync_window(ui.view())?;
    app.set_commands(ui.view());
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
        let browser_owner = app.runtime_owner.clone();
        let (journal, turns, browser) = runtime.block_on(async {
            let storage = TursoAgentStorage::open_local(TursoAgentStorageConfig {
                path: session.path,
                optimization_profile: TursoOptimizationProfile::AsyncIoOnlyCompatibility,
                batch_transaction_mode: TursoBatchTransactionMode::Immediate,
            })
            .await?;
            let project = ProjectId::new(session.project)?;
            let agent = AgentId::new(session.agent)?;
            let browser = SessionBrowser::with_runtime(
                storage.clone(),
                project.clone(),
                agent.clone(),
                browser_owner,
            );
            let (journal, turns) =
                load_session(storage, project, session_id.clone(), agent).await?;
            Ok::<_, Box<dyn std::error::Error>>((journal, turns, browser))
        })?;
        app.session_browser = Some(browser);
        app.current_session = Some(session_id.clone());
        app.transcript
            .push(("Session".into(), session_id.as_str().to_owned()));
        app.restore(turns);
        let bound = JournalGateway::new(gateway, journal).with_tool_offer(
            tool_catalog.as_ref().map_or_else(
                || marlin_agent_runtime::ModelToolCatalog::new().offer(),
                |catalog| catalog.offer(),
            ),
        );
        app.tool_review = bound.tool_review_handle();
        SharedGateway::new(bound)
    } else {
        SharedGateway::new(gateway)
    };
    let signals = SignalMonitor::new(&runtime, &app.runtime_owner)?;
    trace_startup("session and signals");
    let mut terminal = TerminalOwner::open()?;
    let (keys_tx, keys_rx) = mpsc::sync_channel(INPUT_QUEUE_ITEMS);
    let mouse_position = Arc::new(AtomicU64::new(0));
    let input_mouse_position = Arc::clone(&mouse_position);
    let wheel_steps = Arc::new(WheelAccumulator::default());
    let input_wheel_steps = Arc::clone(&wheel_steps);
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
                            TerminalInput::MouseWheel { up, column, row } => {
                                input_mouse_position.store(
                                    (u64::from(row) << 16) | u64::from(column),
                                    Ordering::Relaxed,
                                );
                                if input_wheel_steps.push(up, column, row) {
                                    let _ = keys_tx.try_send(Ok(TerminalInput::WheelWake));
                                }
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
            app.workspace.pulse_host();
            #[cfg(target_os = "macos")]
            if let Some(pulse) = &host_pulse {
                pulse.pulse(marlin_agent_telemetry::host_watchdog::HostHeartbeatSource::Tui);
            }
            if !app.composer.cursor_at_end()
                || (!app.command_selection_locked
                    && app
                        .command_selection
                        .as_ref()
                        .is_some_and(|(query, _)| query != app.composer.text()))
            {
                app.command_selection = None;
                app.command_selection_locked = false;
            }
            if app.command_draft.is_some() {
                if !app.command_query_is_current()
                    || app.command_candidates_pending != Some(app.pending.is_some())
                {
                    let query = app.command_lookup_query();
                    let locked = app
                        .command_selection
                        .as_ref()
                        .filter(|_| app.command_selection_locked)
                        .map(|(_, name)| name.clone());
                    app.command_query = Some(query.clone());
                    app.command_candidates_pending = Some(app.pending.is_some());
                    let (candidates, selected) =
                        if marlin_agent_ui::UiCommandName::is_valid_prefix(&query) {
                            match ui.command_candidates(&query, locked.as_ref()) {
                                Ok(candidates) => (candidates.names, candidates.selected),
                                Err(error) => {
                                    publish_error(
                                        &mut app,
                                        MessageSource::Ui,
                                        "Command candidates failed",
                                        &error,
                                    );
                                    (Vec::new(), None)
                                }
                            }
                        } else {
                            (Vec::new(), None)
                        };
                    let mut candidates = candidates;
                    app.admit_available_commands(&mut candidates);
                    app.command_candidates = candidates;
                    app.command_selection = selected
                        .filter(|name| app.command_candidates.contains(name))
                        .map(|name| (app.composer.text().to_owned(), name));
                    if app.command_selection.is_none() {
                        app.command_selection_locked = false;
                        app.seed_command_selection();
                    }
                    schedule.mark_dirty();
                }
            } else if app.command_query.take().is_some() {
                app.command_candidates_pending = None;
                app.command_candidates.clear();
                app.command_selection = None;
                app.command_selection_locked = false;
            }
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            let processed = process_replies(&mut app, &mut reply_rx, REPLY_BATCH_LIMIT);
            let tool_review_finished = poll_tool_review(&mut app, &gateway, &runtime, &reply_tx);
            let tool_review_opened = open_tool_review(&mut app, &ui);
            let runtime_processed = app.poll_runtime_events(RUNTIME_EVENT_BATCH_LIMIT);
            let sessions_changed = app.poll_sessions();
            let workspace_health_changed = if let Some(report) = app.workspace.poll_health() {
                use marlin_agent_runtime::process::ProcessHealthCondition;
                let (summary, detail) = match report.condition() {
                    ProcessHealthCondition::HostHeartbeatMissed => (
                        "Terminal heartbeat missed",
                        "The terminal event loop missed its heartbeat. The workspace command is still managed.",
                    ),
                    ProcessHealthCondition::TaskOutputIdle => (
                        "Workspace output idle",
                        "The workspace command has not produced output recently. It can continue until its command deadline.",
                    ),
                };
                app.publish_message(MessageEvent {
                    source: MessageSource::Workspace,
                    severity: MessageSeverity::Warning,
                    summary,
                    detail,
                });
                true
            } else {
                false
            };
            let preview_buffer = app.preview_buffer();
            let processes_changed = matches!(preview_buffer.as_ref().unwrap_or(&app.buffer_view), super::BufferView::Named(id) if id.as_str() == "processes")
                && app.refresh_process_content();
            let workspace_changed = if let Some(result) = app.workspace.poll() {
                match result {
                    Ok(view) => app.workspace.show(view),
                    Err(error) => publish_error(
                        &mut app,
                        MessageSource::Workspace,
                        "Workspace read failed",
                        &error,
                    ),
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
            if app.ui_request.is_some()
                || app.command_history_request.is_some()
                || app.transient_request.is_some()
            {
                apply_ui_request(&mut app, &mut ui);
                schedule.mark_dirty();
            }
            if processed > 0
                || tool_review_finished
                || tool_review_opened
                || runtime_processed > 0
                || sessions_changed
                || switched
                || workspace_changed
                || processes_changed
                || workspace_health_changed
                || search_finished
            {
                schedule.mark_dirty();
            }
            let size = termion::terminal_size()?;
            if last_size != Some(size) {
                last_size = Some(size);
                schedule.mark_dirty();
            }
            let mouse = mouse_position.load(Ordering::Relaxed);
            let area = ratatui::layout::Rect::new(0, 0, size.0, size.1);
            let mut geometry = frame_regions(&app, ui.view(), area);
            let hovered = mouse_over_minibuffer(&geometry, mouse as u16, (mouse >> 16) as u16);
            if hovered != app.minibuffer_hovered {
                app.minibuffer_hovered = hovered;
                geometry = frame_regions(&app, ui.view(), area);
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
                        publish_error(
                            &mut app,
                            MessageSource::Ui,
                            "UI label refresh failed",
                            &error,
                        );
                        failed_ui_labels = Some(attempted);
                    } else {
                        failed_ui_labels = None;
                    }
                }
                let mut effective_scroll = app.scroll_up;
                terminal.active().draw(|frame| {
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
                || runtime_processed == RUNTIME_EVENT_BATCH_LIMIT
                || wheel_steps.has_pending()
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
            if let Some((up, pulses, column, row)) = wheel_steps.take_steps() {
                let hit = if column > 0 && row > 0 {
                    super::super::ui_elements::hit_test_scroll(&geometry, column - 1, row - 1)
                } else {
                    None
                };
                let target = hit.and_then(|region| region.scroll_target());
                let units = pulses.saturating_mul(usize::from(
                    hit.and_then(|region| region.scroll_step()).unwrap_or(0),
                ));
                if target == Some("transcript")
                    && let Some(prompt) = app.tool_review_prompt.as_mut()
                {
                    prompt.scroll = if up {
                        prompt
                            .scroll
                            .saturating_sub(u16::try_from(units).unwrap_or(u16::MAX))
                    } else {
                        prompt
                            .scroll
                            .saturating_add(u16::try_from(units).unwrap_or(u16::MAX))
                    };
                } else if target == Some("candidates") && app.tool_review_prompt.is_some() {
                    if let Some(prompt) = app.tool_review_prompt.as_mut() {
                        prompt.selected = if up {
                            prompt.selected.saturating_sub(units)
                        } else {
                            prompt
                                .selected
                                .saturating_add(units)
                                .min(prompt.menu.choices.len().saturating_sub(1))
                        };
                    }
                } else if target == Some("candidates") && command_menu_active(&app, &ui) {
                    browse_menu(
                        &mut app,
                        &ui,
                        if up {
                            UiBrowseDirection::Previous
                        } else {
                            UiBrowseDirection::Next
                        },
                        u16::try_from(units).unwrap_or(u16::MAX),
                    );
                } else if target == Some("transcript") {
                    app.scroll_lines(up, units);
                }
                schedule.mark_dirty();
            }
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                break;
            }
            #[cfg(unix)]
            let requested_suspend = signals.take_suspend_request();
            #[cfg(unix)]
            if requested_suspend || matches!(&received, Ok(Ok(TerminalInput::Key(Key::Ctrl('z')))))
            {
                terminal.suspend()?;
                // SAFETY: SIGSTOP stops this process only after the terminal
                // guards have been dropped. SIGCONT resumes it in the foreground.
                // The SIGTSTP observer remains installed for external job control.
                if unsafe { libc::raise(libc::SIGSTOP) } != 0 {
                    return Err(io::Error::last_os_error());
                }
                terminal.resume()?;
                last_size = None;
                schedule.mark_dirty();
                continue;
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
                if let Some(prompt) = app.tool_review_prompt.as_mut() {
                    match key {
                        Key::Up => prompt.selected = prompt.selected.saturating_sub(1),
                        Key::Down => {
                            prompt.selected =
                                (prompt.selected + 1).min(prompt.menu.choices.len() - 1)
                        }
                        Key::PageUp => {
                            prompt.scroll = prompt.scroll.saturating_sub(size.1.saturating_sub(4))
                        }
                        Key::PageDown => {
                            prompt.scroll = prompt.scroll.saturating_add(size.1.saturating_sub(4))
                        }
                        Key::Char('\n') => submit_tool_review(&mut app, &runtime),
                        Key::Ctrl('q') => break,
                        _ => {}
                    }
                    continue;
                }
                let menu_active = command_menu_active(&app, &ui);
                if menu_active && matches!(key, Key::Up | Key::Down) {
                    let direction = if *key == Key::Up {
                        UiBrowseDirection::Previous
                    } else {
                        UiBrowseDirection::Next
                    };
                    browse_menu(&mut app, &ui, direction, 1);
                    continue;
                }
                if *key == Key::Char('\t') && menu_active {
                    let completed = app
                        .selected_command()
                        .and_then(|name| {
                            ui.complete_selected_command(&app.command_candidates, name)
                        })
                        .or_else(|| {
                            ui.complete_command(
                                &app.command_lookup_query(),
                                &app.command_candidates,
                            )
                        });
                    if let Some(completed) = completed {
                        app.accept_command_completion(&completed);
                    }
                    app.command_selection = None;
                    app.command_selection_locked = false;
                    continue;
                }
                if *key == Key::Char('\n') && menu_active {
                    match ui.accept_command(
                        &app.command_lookup_query(),
                        &app.command_candidates,
                        app.selected_command(),
                    ) {
                        Ok(Some(completed)) => {
                            if app.accept_command_completion(&completed) {
                                app.submit(&gateway, &runtime, &reply_tx);
                                apply_ui_request(&mut app, &mut ui);
                            }
                        }
                        Ok(None) => {}
                        Err(error) => publish_error(
                            &mut app,
                            MessageSource::Ui,
                            "Command menu failed",
                            &error,
                        ),
                    }
                    continue;
                }
                let routed = ui.route_key(
                    *key,
                    app.command_draft.is_some(),
                    app.command_draft.is_some(),
                    app.key_mode,
                );
                app.transient_hint = ui.transient_hint().cloned();
                match routed {
                    Ok(UiKeyAction::Handled) => continue,
                    Ok(UiKeyAction::Cancel) => {
                        app.cancel();
                        apply_ui_request(&mut app, &mut ui);
                        continue;
                    }
                    Ok(UiKeyAction::Mode { mode, cancel }) => {
                        // Escape first preserves existing cancel/return behaviour.
                        // Enter normal mode only from the idle conversation.
                        let enter_mode = !cancel
                            || (app.buffer_view == super::BufferView::Conversation
                                && app.modeline_activity() == super::ModelineActivity::Idle);
                        if cancel {
                            app.cancel();
                            apply_ui_request(&mut app, &mut ui);
                        }
                        if enter_mode {
                            if mode == marlin_agent_ui::UiKeyMode::Normal
                                && app.key_mode == marlin_agent_ui::UiKeyMode::Insert
                            {
                                app.composer.normal_cursor();
                            }
                            app.key_mode = mode;
                        }
                        continue;
                    }
                    Ok(UiKeyAction::Edit(action)) => {
                        let accepts_text = ui.accepts_text(app.command_draft.is_some());
                        let normal = app.key_mode == marlin_agent_ui::UiKeyMode::Normal
                            && app.command_draft.is_none();
                        match action {
                            UiEditorAction::HistorySearch => app.start_history_search(),
                            UiEditorAction::PreviousPrompt if accepts_text => {
                                app.navigate_prompts(true)
                            }
                            UiEditorAction::NextPrompt if accepts_text => {
                                app.navigate_prompts(false)
                            }
                            UiEditorAction::InsertNewline if accepts_text => {
                                app.composer.insert('\n')
                            }
                            UiEditorAction::BackwardChar if accepts_text && normal => {
                                app.composer.normal_left()
                            }
                            UiEditorAction::BackwardChar if accepts_text => app.composer.left(),
                            UiEditorAction::ForwardChar if accepts_text && normal => {
                                app.composer.normal_right()
                            }
                            UiEditorAction::ForwardChar if accepts_text => app.composer.right(),
                            UiEditorAction::DeleteChar if accepts_text => {
                                app.composer.delete();
                                if normal {
                                    app.composer.clamp_normal_cursor();
                                }
                            }
                            UiEditorAction::PreviousLine if accepts_text => {
                                app.composer.move_vertical(size.0.saturating_sub(2), false);
                                if normal {
                                    app.composer.clamp_normal_cursor();
                                }
                            }
                            UiEditorAction::NextLine if accepts_text => {
                                app.composer.move_vertical(size.0.saturating_sub(2), true);
                                if normal {
                                    app.composer.clamp_normal_cursor();
                                }
                            }
                            UiEditorAction::PreviousLine => app.scroll(true),
                            UiEditorAction::NextLine => app.scroll(false),
                            UiEditorAction::NextWord if accepts_text => {
                                app.composer.next_word();
                                if normal {
                                    app.composer.clamp_normal_cursor();
                                }
                            }
                            UiEditorAction::PreviousWord if accepts_text => {
                                app.composer.previous_word()
                            }
                            UiEditorAction::DeleteLine if accepts_text => {
                                app.composer.delete_line();
                                app.composer.clamp_normal_cursor();
                            }
                            UiEditorAction::Undo if accepts_text => {
                                app.composer.undo();
                                app.composer.clamp_normal_cursor();
                            }
                            UiEditorAction::AppendInsert if accepts_text => {
                                app.composer.right();
                                app.key_mode = marlin_agent_ui::UiKeyMode::Insert;
                            }
                            UiEditorAction::LineStart if accepts_text => app.composer.line_start(),
                            UiEditorAction::LineEnd if accepts_text => {
                                app.composer.line_end();
                                if normal {
                                    app.composer.clamp_normal_cursor();
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    Ok(UiKeyAction::InvokeCommand(name)) => {
                        if app.command_draft.is_none() {
                            app.toggle_command();
                        }
                        app.command_menu = None;
                        app.composer.take();
                        app.composer.insert_text(name.as_str());
                        app.command_selection = None;
                        app.command_selection_locked = false;
                        app.submit(&gateway, &runtime, &reply_tx);
                        apply_ui_request(&mut app, &mut ui);
                        continue;
                    }
                    Ok(UiKeyAction::Submit) => {
                        app.submit(&gateway, &runtime, &reply_tx);
                        apply_ui_request(&mut app, &mut ui);
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
                        publish_error(&mut app, MessageSource::Ui, "UI event failed", &error);
                        continue;
                    }
                }
            }
            let accepts_text = ui.accepts_text(app.command_draft.is_some());
            match received {
                Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) if app.interrupt() => break,
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) => {}
                Ok(Ok(TerminalInput::Key(Key::Esc))) => app.cancel(),
                Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() && accepts_text => {
                    app.composer.insert(c)
                }
                Ok(Ok(TerminalInput::Key(Key::Left))) if accepts_text => app.composer.left(),
                Ok(Ok(TerminalInput::Key(Key::Right))) if accepts_text => app.composer.right(),
                Ok(Ok(TerminalInput::Key(Key::Home))) if accepts_text => app.composer.line_start(),
                Ok(Ok(TerminalInput::Key(Key::End))) if accepts_text => app.composer.line_end(),
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
                Ok(Ok(TerminalInput::WheelWake)) => {}
                Ok(Ok(TerminalInput::Paste(text))) if accepts_text => {
                    app.composer.insert_text(&text)
                }
                Ok(Ok(TerminalInput::PasteRejected)) => app.publish_message(MessageEvent {
                    source: MessageSource::Input,
                    severity: MessageSeverity::Warning,
                    summary: "Paste rejected",
                    detail: "Invalid UTF-8 or larger than 1 MiB.",
                }),
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                _ => {}
            }
            apply_ui_request(&mut app, &mut ui);
        }
        Ok(())
    })();
    trace_startup("event loop exit");
    let terminal_result = terminal.suspend();
    drop(signals);
    #[cfg(target_os = "macos")]
    if let Some(task) = heartbeat_task {
        task.abort();
    }
    let shutdown_result = runtime.block_on(app.shutdown());
    drop(runtime_probe);
    trace_startup("terminal restored");
    if env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some()
        && let Some(elapsed) = first_frame_elapsed
    {
        eprintln!("marlin-tui startup first frame: {elapsed:?}");
    }
    drop(app);
    drop(gateway);
    drop(base_gateway);
    trace_startup("gateway dropped");
    drop(runtime);
    trace_startup("Tokio runtime dropped");
    drop(ui);
    trace_startup("UI session dropped");
    drop(scheme_runtime);
    trace_startup("Scheme VM dropped");
    terminal_result?;
    shutdown_result?;
    result?;
    Ok(signal_exit)
}

fn start_telemetry() -> Result<Option<OpenTelemetry>, Box<dyn std::error::Error>> {
    let endpoint = match env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
        Ok(endpoint) => endpoint,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let telemetry = OpenTelemetry::otlp_http(
        TelemetryServiceName::new("marlin-tui"),
        TraceTarget::new(marlin_agent_runtime::observability::TARGET_RUNTIME_SPAN),
        &endpoint,
    )?;
    telemetry.install_global()?;
    Ok(Some(telemetry))
}

/// Start the terminal client with the process argument list.
pub fn run_cli() {
    #[cfg(target_os = "macos")]
    if marlin_agent_telemetry::run_host_observer_if_requested() {
        return;
    }
    let args = match parse_cli(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let telemetry = match start_telemetry() {
        Ok(telemetry) => telemetry,
        Err(error) => {
            eprintln!("marlin-tui: telemetry configuration: {error}");
            std::process::exit(2);
        }
    };
    let result = match args.action {
        CliAction::Open { session } => run(args.config, session),
        CliAction::List { after } => list_sessions(&args.config, after).map(|()| None),
    };
    if let Some(telemetry) = telemetry
        && let Err(error) = telemetry.shutdown()
    {
        eprintln!("marlin-tui: telemetry shutdown: {error}");
    }
    match result {
        Ok(Some(code)) => std::process::exit(code),
        Ok(None) => {}
        Err(error) => {
            let _ = writeln!(io::stderr(), "marlin-tui: {error}");
            std::process::exit(1);
        }
    }
}
