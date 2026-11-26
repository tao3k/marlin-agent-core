// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal event loop and process entry.

use super::{
    AgentId, App, Arc, AtomicU64, FrameSchedule, HistorySearch, Instant, JournalGateway, Key,
    MessageEvent, MessageSeverity, MessageSource, Ordering, ProjectId, REPLY_BATCH_LIMIT,
    REPLY_QUEUE_BYTES, REPLY_QUEUE_ITEMS, RUNTIME_EVENT_BATCH_LIMIT, RuntimeEdgePolicy,
    RuntimeStreamGateway, ScrollAnchor, SessionBrowser, SessionId, SharedGateway, SignalMonitor,
    TerminalInput, TerminalOwner, TranscriptViewportCache, TursoAgentStorage,
    TursoAgentStorageConfig, TursoBatchTransactionMode, TursoOptimizationProfile,
    UiBrowseDirection, UiHost, UiKeyAction, UiOpenFacts, WheelAccumulator, config,
    draw_with_snapshot, frame_snapshot, io, load_session, mouse_over_minibuffer, mpsc,
    publish_error, reply_channel,
};
use marlin_agent_ui::{UiEditorAction, UiPackageCommandSource};
use std::{
    env,
    io::{IsTerminal, Write},
    path::PathBuf,
};

use super::runtime_probe::RuntimeProbe;
use super::tool_review::{open_tool_review, poll_tool_review, submit_tool_review};
use super::ui_events::{
    activate_deferred_command, apply_editor_mode, apply_ui_request, apply_wheel_scroll,
    browse_menu, command_menu_active, exit_reason_for_input_error, handle_helm_input,
    handle_history_search_input, release_interrupted_key_sequence, route_frame_click,
    route_terminal_key, spawn_stdin_reader, stage_ui_package, unexpected_stdin_eof,
};

pub(super) fn run(
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
    #[cfg(feature = "linked-ui")]
    let git_context_approval = config
        .git_context
        .as_ref()
        .map(|approval| approval.repository.clone());
    #[cfg(not(feature = "linked-ui"))]
    if config.git_context.is_some() {
        return Err("Git Context requires the linked Scheme programme".into());
    }
    #[cfg(not(target_os = "macos"))]
    if config.process_budget.is_some() {
        return Err("process memory sampling is unavailable on this host".into());
    }
    if selected_session.is_some() && config.session.is_none() {
        return Err("--session requires a [session] storage configuration".into());
    }
    let (endpoint, catalog) = config.catalog()?;
    trace_startup("configuration");
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
        method_factories: &[],
    };
    #[cfg(feature = "linked-ui")]
    // SAFETY: the binary links these exports from the archive built from the
    // same selected Scheme programme as `linked_ui::initialize`.
    let ui_module = unsafe {
        marlin_agent_ui::UiModule::resolve_linked(
            &scheme_runtime,
            super::super::linked_ui::exports(),
        )?
    };
    #[cfg(not(feature = "linked-ui"))]
    let ui_module = marlin_agent_ui::UiModule::resolve(&scheme_runtime, directory)?;
    let mut ui_package = config.tui_ui_package;
    let mut ui = UiHost::from_module(ui_module, &ui_facts)?;
    if let Some(status) = status.first()
        && !ui.view().regions.iter().any(|region| {
            matches!(region, marlin_agent_ui::UiRegion::Status(items) if items.iter().any(|item| item == status))
        })
    {
        return Err("Scheme projection lost the Runtime budget status".into());
    }
    trace_startup("UI session projection");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    trace_startup("Tokio runtime");
    let directory = env::current_dir()?;
    #[cfg(feature = "linked-ui")]
    let (search_dsl_runner, search_dsl_mailbox) = super::super::linked_ui::search_dsl_channel();
    #[cfg(feature = "linked-ui")]
    let search_tool_slot = marlin_agent_tooling::SharedSearchIndexSlot::empty();
    #[cfg(feature = "linked-ui")]
    search_tool_slot.attach_dsl_runner(search_dsl_runner);
    #[cfg(feature = "linked-ui")]
    let tool_catalog = if config.session.is_some() {
        let declarations = super::super::linked_ui::tooling_declarations(&scheme_runtime)?;
        Some(Arc::new(
            marlin_agent_tooling::catalog_from_declarations_with_search(
                declarations,
                directory.clone(),
                search_tool_slot.clone(),
            )
            .map_err(|error| format!("cannot bind Scheme tool catalog: {error:?}"))?,
        ))
    } else {
        None
    };
    #[cfg(not(feature = "linked-ui"))]
    let tool_catalog: Option<Arc<marlin_agent_runtime::ModelToolCatalog>> = None;
    trace_startup("tool catalog binding");
    let envrc_discovery = marlin_agent_runtime::ProjectEnvrcDiscovery::pending();
    let request = marlin_agent_runtime::TokioAgentRuntimeBuildRequest::runtime_root(64)
        .with_project_envrc_discovery(envrc_discovery.clone());
    #[cfg(target_os = "macos")]
    let request = match config.process_budget {
        Some(budget) => request.with_process_budget(
            budget,
            Arc::new(marlin_agent_telemetry::MacOsProcessMemorySampler),
        ),
        None => request,
    };
    let (owner, events) = marlin_agent_runtime::TokioAgentRuntime::from_build_request(request);
    let mut app = App::with_runtime(endpoint, owner, events);
    #[cfg(feature = "linked-ui")]
    let launch_checkout = directory.clone();
    let mut worktree_probe = Some(app.runtime_owner.spawn_on(&runtime, async move {
        super::envrc::project(&directory).await
    }));
    let mut envrc_status = None;
    app.transcript_policy = Some(ui.view().layout.transcript_policy.clone());
    app.tool_offer_options = tool_catalog
        .as_ref()
        .map(|catalog| catalog.offer().completion_options());
    app.tool_catalog = tool_catalog.clone();
    #[cfg(feature = "linked-ui")]
    {
        app.search_tool_slot = search_tool_slot;
    }
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
    let mouse_position = Arc::new(AtomicU64::new(0));
    let wheel_steps = Arc::new(WheelAccumulator::default());
    let keys_rx = spawn_stdin_reader(Arc::clone(&mouse_position), Arc::clone(&wheel_steps));
    let (reply_tx, mut reply_rx) = reply_channel(REPLY_QUEUE_ITEMS, REPLY_QUEUE_BYTES);
    let mut schedule = FrameSchedule::new();
    let mut viewport_cache = TranscriptViewportCache::default();
    let mut scroll_anchor = ScrollAnchor::default();
    let mut painted_regions = Vec::new();
    let mut layout_snapshot = None;
    let mut last_size = None;
    let mut first_frame_elapsed = None;
    let mut failed_ui_labels: Option<(String, String)> = None;
    let mut signal_exit = None;
    let mut exit_reason = "unknown";
    #[cfg(feature = "linked-ui")]
    let mut git_context: Option<super::git_context::GitContext<'_>> = None;
    let result = (|| -> io::Result<()> {
        loop {
            if let Some(branch) = app.worktree_switch.take() {
                #[cfg(feature = "linked-ui")]
                if let Some(context) = git_context.as_mut() {
                    context.switch(&mut app, &runtime, branch);
                    schedule.mark_dirty();
                    apply_ui_request(&mut app, &mut ui);
                    continue;
                }
                let _ = branch;
                app.publish_message(MessageEvent {
                    source: MessageSource::Workspace,
                    severity: MessageSeverity::Warning,
                    summary: "Worktree switch unavailable",
                    detail: "Configure an approved repository in [git_context].",
                });
                schedule.mark_dirty();
            }
            #[cfg(feature = "linked-ui")]
            if git_context
                .as_mut()
                .is_some_and(|context| context.poll(&mut app, &runtime))
            {
                schedule.mark_dirty();
            }
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
                if stage_ui_package(&mut app, &mut ui, &mut ui_package) {
                    schedule.mark_dirty();
                }
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
                exit_reason = "signal";
                break;
            }
            let processed = app.process_replies(&mut reply_rx, REPLY_BATCH_LIMIT);
            let tool_review_finished = poll_tool_review(&mut app, &gateway, &runtime, &reply_tx);
            let tool_review_opened = open_tool_review(&mut app, &ui);
            let runtime_processed = app.poll_runtime_events(RUNTIME_EVENT_BATCH_LIMIT);
            if worktree_probe
                .as_ref()
                .is_some_and(|probe| probe.is_finished())
            {
                let probe = worktree_probe.take().expect("finished worktree discovery");
                let binding = runtime.block_on(probe.join()).ok().flatten();
                if let Some(binding) = &binding {
                    let _entered = runtime.enter();
                    super::envrc::submit(
                        &app.runtime_owner,
                        &scheme_runtime,
                        binding,
                        marlin_agent_runtime::EnvrcIntent::Activate,
                    );
                }
                // Publish after setting Loading so waiting tools cannot
                // mistake an undiscovered environment for an absent file.
                envrc_discovery.complete(binding);
            }
            let envrc_changed = super::envrc::report_changes(&mut app, &mut envrc_status);
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
            let helm_updated = app.poll_search_index();
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
                || envrc_changed
                || sessions_changed
                || switched
                || workspace_changed
                || processes_changed
                || workspace_health_changed
                || search_finished
                || helm_updated
            {
                schedule.mark_dirty();
            }
            if app.update_minibuffer(Instant::now()) {
                schedule.mark_dirty();
            }
            let size = termion::terminal_size()?;
            if last_size != Some(size) {
                last_size = Some(size);
                schedule.mark_dirty();
            }
            let mouse = mouse_position.load(Ordering::Relaxed);
            let area = ratatui::layout::Rect::new(0, 0, size.0, size.1);
            // Idle polls and pointer hit checks can reuse the last layout.
            let snapshot = match layout_snapshot.as_mut() {
                Some(existing) if !schedule.has_changes() => existing,
                _ => layout_snapshot.insert(frame_snapshot(&app, ui.view(), area)),
            };
            let hovered = mouse_over_minibuffer(
                &snapshot.regions,
                ui.view()
                    .layout
                    .regions
                    .id(marlin_agent_ui::UiFrameRole::MinibufferHover),
                mouse as u16,
                (mouse >> 16) as u16,
            );
            if hovered != app.minibuffer_hovered {
                app.minibuffer_hovered = hovered;
                *snapshot = frame_snapshot(&app, ui.view(), area);
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
                        *snapshot = frame_snapshot(&app, ui.view(), area);
                    } else {
                        failed_ui_labels = None;
                        *snapshot = frame_snapshot(&app, ui.view(), area);
                    }
                }
                let mut effective_scroll = app.scroll_up;
                terminal.active().draw(|frame| {
                    if snapshot.area != frame.area() {
                        *snapshot = frame_snapshot(&app, ui.view(), frame.area());
                    }
                    let painted = draw_with_snapshot(
                        frame,
                        &app,
                        ui.view(),
                        &mut viewport_cache,
                        &mut scroll_anchor,
                        snapshot,
                    );
                    effective_scroll = painted.scroll;
                    painted_regions = painted.regions;
                })?;
                app.set_effective_scroll(effective_scroll);
                let drawn = Instant::now();
                schedule.drawn(drawn);
                app.minibuffer_drawn(drawn);
                if first_frame_elapsed.is_none() {
                    first_frame_elapsed = Some(startup.elapsed());
                    #[cfg(feature = "linked-ui")]
                    {
                        let helm_cfg = super::super::linked_ui::helm_index_config(&scheme_runtime)
                            .unwrap_or_default();
                        let _ = super::super::linked_ui::sync_asp_provider_descriptors_from_env(
                            &scheme_runtime,
                        );
                        app.search_dsl_catalog =
                            super::super::linked_ui::search_dsl_completions(&scheme_runtime, "")
                                .unwrap_or_default();
                        app.warm_search_index(helm_cfg, runtime.handle());
                        if let Some(approved) = git_context_approval.as_ref() {
                            git_context = Some(super::git_context::GitContext::new(
                                &scheme_runtime,
                                approved.clone(),
                                launch_checkout.clone(),
                                &app,
                                &runtime,
                            ));
                        }
                    }
                    #[cfg(not(feature = "linked-ui"))]
                    app.warm_search_index(
                        marlin_agent_search_engine::WorkspaceIndexConfig::default(),
                        runtime.handle(),
                    );
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
            } else if app
                .helm
                .as_ref()
                .is_some_and(super::HelmOverlay::is_indexing)
            {
                schedule
                    .wait_duration(Instant::now())
                    .min(std::time::Duration::from_millis(8))
            } else {
                schedule.wait_duration(Instant::now())
            };
            let received = keys_rx.recv_timeout(wait);
            #[cfg(feature = "linked-ui")]
            {
                search_dsl_mailbox.drain(&scheme_runtime);
                if super::super::linked_ui::service_helm_dsl(&scheme_runtime, &mut app) {
                    schedule.mark_dirty();
                }
            }
            if let Some((up, pulses, column, row)) = wheel_steps.take_steps() {
                apply_wheel_scroll(&mut app, &ui, &painted_regions, up, pulses, column, row);
                schedule.mark_dirty();
            }
            signal_exit = signals.observed_exit_code();
            if signal_exit.is_some() {
                exit_reason = "signal";
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
            if let Ok(Ok(input)) = &received {
                schedule.mark_dirty();
                if matches!(
                    input,
                    TerminalInput::Key(_)
                        | TerminalInput::Paste(_)
                        | TerminalInput::PasteRejected
                        | TerminalInput::MouseClick(_, _)
                ) {
                    app.echo_area.on_input();
                }
            }
            if matches!(&received, Ok(Ok(input)) if !matches!(input, TerminalInput::Key(Key::Ctrl('c'))))
            {
                app.disarm_exit();
            }
            release_interrupted_key_sequence(
                &mut app,
                &mut ui,
                received.as_ref().ok().and_then(|input| input.as_ref().ok()),
            );
            if let Ok(Ok(TerminalInput::MouseClick(column, row))) = &received {
                if route_frame_click(&mut app, &mut ui, &painted_regions, *column, *row)
                    && activate_deferred_command(&mut app, &mut ui, UiPackageCommandSource::Command)
                {
                    app.submit(&gateway, &runtime, &reply_tx);
                    apply_ui_request(&mut app, &mut ui);
                }
                continue;
            }
            if app.helm.is_some() {
                match handle_helm_input(&mut app, &mut ui, runtime.handle(), received) {
                    Ok(true) => {
                        exit_reason = "ctrl-q";
                        break;
                    }
                    Ok(false) => continue,
                    Err(error) => {
                        exit_reason = exit_reason_for_input_error(&error);
                        return Err(error);
                    }
                }
            }
            if app.history_search.is_some() {
                match handle_history_search_input(&mut app, received) {
                    Ok(true) => {
                        exit_reason = "ctrl-q";
                        break;
                    }
                    Ok(false) => continue,
                    Err(error) => {
                        exit_reason = exit_reason_for_input_error(&error);
                        return Err(error);
                    }
                }
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
                        Key::Ctrl('q') => {
                            exit_reason = "ctrl-q";
                            break;
                        }
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
                            if app.accept_command_completion(&completed)
                                && activate_deferred_command(
                                    &mut app,
                                    &mut ui,
                                    UiPackageCommandSource::Command,
                                )
                            {
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
                let mut routed = route_terminal_key(&mut app, &mut ui, *key);
                if ui_package.is_some()
                    && app.command_draft.is_none()
                    && (ui.has_undefined_key_sequence()
                        || (matches!(routed, Ok(UiKeyAction::Unhandled))
                            && matches!(key, Key::Alt(_))))
                    && stage_ui_package(&mut app, &mut ui, &mut ui_package)
                {
                    // The built-in map has already declined the key. Replay
                    // it with the same prefix in the declaration's keymap.
                    ui.retry_undefined_key_sequence();
                    routed = route_terminal_key(&mut app, &mut ui, *key);
                }
                match routed {
                    Ok(UiKeyAction::Handled) => continue,
                    Ok(UiKeyAction::Cancel) => {
                        if app.command_draft.is_none()
                            && ui.accepts_text(false)
                            && app.composer.clear_selection()
                        {
                            app.composer.clamp_normal_cursor();
                            continue;
                        }
                        app.cancel();
                        apply_ui_request(&mut app, &mut ui);
                        continue;
                    }
                    Ok(UiKeyAction::Mode(mode)) => {
                        apply_editor_mode(&mut app, &mut ui, mode);
                        continue;
                    }
                    Ok(UiKeyAction::Edit(action)) => {
                        app.edit(
                            action,
                            snapshot.editor_width,
                            ui.accepts_text(app.command_draft.is_some()),
                        );
                        continue;
                    }
                    Ok(UiKeyAction::InvokeCommand(name)) => {
                        if app.prepare_bound_command(&name)
                            && activate_deferred_command(
                                &mut app,
                                &mut ui,
                                UiPackageCommandSource::KeyBinding,
                            )
                        {
                            app.submit(&gateway, &runtime, &reply_tx);
                            apply_ui_request(&mut app, &mut ui);
                        }
                        continue;
                    }
                    Ok(UiKeyAction::Submit) => {
                        if activate_deferred_command(
                            &mut app,
                            &mut ui,
                            UiPackageCommandSource::Command,
                        ) {
                            app.submit(&gateway, &runtime, &reply_tx);
                            apply_ui_request(&mut app, &mut ui);
                        }
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
                Ok(Ok(TerminalInput::Key(Key::Ctrl('q')))) => {
                    exit_reason = "ctrl-q";
                    break;
                }
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) if app.interrupt() => {
                    exit_reason = "ctrl-c";
                    break;
                }
                Ok(Ok(TerminalInput::Key(Key::Ctrl('c')))) => {}
                Ok(Ok(TerminalInput::Key(Key::Esc))) => app.cancel(),
                Ok(Ok(TerminalInput::Key(Key::Char(c)))) if !c.is_control() && accepts_text => {
                    app.composer.insert(c)
                }
                Ok(Ok(TerminalInput::Key(Key::Left))) if accepts_text => {
                    app.edit(UiEditorAction::BackwardChar, size.0, true)
                }
                Ok(Ok(TerminalInput::Key(Key::Right))) if accepts_text => {
                    app.edit(UiEditorAction::ForwardChar, size.0, true)
                }
                Ok(Ok(TerminalInput::Key(Key::Home))) if accepts_text => {
                    app.edit(UiEditorAction::LineStart, size.0, true)
                }
                Ok(Ok(TerminalInput::Key(Key::End))) if accepts_text => {
                    app.edit(UiEditorAction::LineEnd, size.0, true)
                }
                Ok(Ok(TerminalInput::Key(Key::Up))) if accepts_text => {
                    app.edit(UiEditorAction::PreviousLine, snapshot.editor_width, true)
                }
                Ok(Ok(TerminalInput::Key(Key::Down))) if accepts_text => {
                    app.edit(UiEditorAction::NextLine, snapshot.editor_width, true)
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
                Ok(Err(e)) => {
                    exit_reason = "input-error";
                    return Err(e);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    exit_reason = "stdin-eof";
                    return Err(unexpected_stdin_eof());
                }
                _ => {}
            }
            if let Some(command) = app.envrc_command.take() {
                super::envrc::apply(&mut app, &runtime, &scheme_runtime, command);
                schedule.mark_dirty();
            }
            apply_ui_request(&mut app, &mut ui);
        }
        Ok(())
    })();
    if env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some() {
        eprintln!(
            "marlin-tui startup event loop exit ({exit_reason}, signal={signal_exit:?}): {:?}",
            startup.elapsed()
        );
    }
    #[cfg(feature = "linked-ui")]
    drop(git_context);
    let terminal_result = terminal.suspend();
    trace_startup("terminal restored");
    drop(signals);
    // Startup discovery is no longer useful after terminal exit. Its Git
    // child is killed on drop, and the tracked task joins the normal drain.
    if let Some(probe) = worktree_probe {
        probe.abort();
    }
    #[cfg(target_os = "macos")]
    if let Some(task) = heartbeat_task {
        task.abort();
    }
    let shutdown_result = runtime.block_on(app.shutdown());
    drop(runtime_probe);
    trace_startup("Runtime tasks drained");
    if env::var_os("MARLIN_TUI_STARTUP_TRACE").is_some()
        && let Some(elapsed) = first_frame_elapsed
    {
        eprintln!("marlin-tui startup first frame: {elapsed:?}");
    }
    drop(app);
    drop(gateway);
    drop(base_gateway);
    trace_startup("gateway dropped");
    runtime.shutdown_timeout(std::time::Duration::from_millis(250));
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
