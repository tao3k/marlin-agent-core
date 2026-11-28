// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Managed runtime process inspection and control commands.

use super::{App, Command, MessageEvent, MessageSeverity, MessageSource, UiContent, UiEvent};

impl App {
    pub(super) fn submit_runtime_command(&mut self) -> bool {
        let Some(_) = self.command_draft else {
            return false;
        };
        let input = match &self.command_menu {
            Some(menu) => format!("{}{}", menu.prefix, self.composer.text()),
            None => self.composer.text().to_owned(),
        };
        let descriptor = self.command_descriptor(&input).cloned();
        let command = descriptor
            .as_ref()
            .and_then(|item| Command::parse(item, &input));
        match command {
            Some(Command::SelectUi(UiEvent::SelectBuffer(buffer)))
                if buffer.as_str() == "processes" =>
            {
                self.ui_request = Some(UiEvent::SelectBuffer(buffer));
                self.refresh_process_content();
            }
            Some(Command::KillProcess(generation)) => self.stop_process(generation),
            Some(Command::Envrc(command)) => self.envrc_command = Some(command),
            _ => return false,
        }
        self.command_history_request = descriptor.map(|item| item.name);
        self.composer = self
            .command_draft
            .take()
            .expect("runtime command has a saved draft");
        self.command_menu = None;
        true
    }

    fn stop_process(&mut self, generation: std::num::NonZeroU64) {
        let session = self.runtime_session_id();
        let stopped = self
            .runtime_owner
            .process_registry()
            .stop_session_generation(&session, generation);
        let (severity, summary, detail) = if stopped {
            (
                MessageSeverity::Info,
                "Process stop requested",
                "Runtime is reaping the selected process.",
            )
        } else {
            (
                MessageSeverity::Warning,
                "Process unavailable",
                "Use a live generation from this session's Processes buffer.",
            )
        };
        self.publish_message(MessageEvent {
            source: MessageSource::Runtime,
            severity,
            summary,
            detail,
        });
    }

    pub(crate) fn refresh_process_content(&mut self) -> bool {
        let session = self.runtime_session_id();
        let registry = self.runtime_owner.process_registry();
        let revision = registry.managed_revision();
        let snapshot_key = (
            session.clone(),
            revision,
            self.tool_executions_revision,
            self.asp_live_status.provider_count,
            self.asp_live_status.hydrated_entities,
        );
        if self.process_snapshot.as_ref() == Some(&snapshot_key) {
            return false;
        }
        let rows = registry.managed_processes(&session);
        let survived = registry.survived_processes();
        let mut content =
            Vec::with_capacity(rows.len() + survived.len() + self.tool_executions.len() * 6 + 12);
        if self.asp_live_status.connected {
            let mode = if self.asp_live_status.managed {
                "managed"
            } else {
                "external"
            };
            content.push(UiContent::Text("* ASP Semantic Gateway".into()));
            content.push(UiContent::Field {
                label: "  Mode".into(),
                value: format!("{mode} (boot {}µs)", self.asp_live_status.boot_micros),
            });
            content.push(UiContent::Field {
                label: "  Providers".into(),
                value: format!(
                    "{} [{}]",
                    self.asp_live_status.provider_count,
                    self.asp_live_status.providers.join(", ")
                ),
            });
            content.push(UiContent::Field {
                label: "  Hydrated AST".into(),
                value: format!("{} entities", self.asp_live_status.hydrated_entities),
            });
        }
        if !self.tool_executions.is_empty() {
            content.push(UiContent::Text(
                "* Tool Executions · OpenTelemetry (span: runtime.tool.execute)".into(),
            ));
            for rec in self.tool_executions.iter().rev() {
                let dur = rec.duration_ms.map_or_else(
                    || format!("{}ms…", rec.started_at.elapsed().as_millis()),
                    |ms| format!("{ms}ms"),
                );
                content.push(UiContent::Text(format!(
                    "** {} {} [{}] ({dur})",
                    rec.state, rec.tool_name, rec.call_id
                )));
                content.push(UiContent::Field {
                    label: "   OTel Span".into(),
                    value: rec.otel_span.to_owned(),
                });
                for (key, value) in &rec.arguments {
                    content.push(UiContent::Field {
                        label: format!("   Arg:{key}"),
                        value: value.clone(),
                    });
                }
                content.push(UiContent::Field {
                    label: "   Summary".into(),
                    value: rec.output_summary.clone(),
                });
                for line in rec.output_lines.iter().take(12) {
                    content.push(UiContent::Text(format!("   | {line}")));
                }
            }
        }
        if rows.is_empty()
            && survived.is_empty()
            && self.tool_executions.is_empty()
            && !self.asp_live_status.connected
        {
            content.push(UiContent::Text(
                "No managed processes or tool executions in this session.".into(),
            ));
        } else if !rows.is_empty() {
            content.push(UiContent::Text("* Managed OS Processes".into()));
            content.push(UiContent::Text(
                "| Generation | State | PID | Operation |".into(),
            ));
        }
        for row in rows {
            let id = row.resource_id().expect("managed process row");
            let operation: String = id
                .operation()
                .as_str()
                .chars()
                .take(128)
                .map(|ch| if ch.is_control() { '\u{FFFD}' } else { ch })
                .collect();
            use marlin_agent_runtime::process::RuntimeProcessStatus;
            let state = match row.status {
                RuntimeProcessStatus::Running => "Running",
                RuntimeProcessStatus::Finished => "Exited",
                RuntimeProcessStatus::Failed => "Exited (failed)",
                RuntimeProcessStatus::Orphaned => "Orphaned",
                RuntimeProcessStatus::CleanupRequested => "Cleaning up",
            };
            content.push(UiContent::Text(format!(
                "{}   {}   {}   {}",
                id.generation().get(),
                state,
                row.pid,
                operation
            )));
        }
        if !survived.is_empty() {
            content.push(UiContent::Text(
                "Survived from previous Marlin run(s): PID   PGID   Descendants   Owner".into(),
            ));
            for item in survived {
                let pgid = item
                    .entry
                    .pgid
                    .map(|p| p.as_u32().to_string())
                    .unwrap_or_else(|| "-".to_owned());
                let owner: String = item
                    .entry
                    .owner_reference
                    .chars()
                    .take(128)
                    .map(|ch| if ch.is_control() { '?' } else { ch })
                    .collect();
                content.push(UiContent::Text(format!(
                    "{}   {}   {}   {} (from {})",
                    item.entry.pid,
                    pgid,
                    item.surviving_descendant_pids.len(),
                    owner,
                    item.entry.host_run_id
                )));
            }
        }
        let id = marlin_agent_ui::UiBufferId::new("processes").expect("static Buffer identity");
        match self.replace_dynamic_buffer_content(id, content) {
            Ok(changed) => {
                self.process_snapshot = Some(snapshot_key);
                changed
            }
            Err(detail) => {
                self.process_snapshot = Some(snapshot_key);
                self.publish_message(MessageEvent {
                    source: MessageSource::Runtime,
                    severity: MessageSeverity::Error,
                    summary: "Process Buffer update rejected",
                    detail,
                });
                false
            }
        }
    }
}
