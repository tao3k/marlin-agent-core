// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bind host-owned content and cached transcripts to declarative UI slots.

use super::{App, MainSource, ScrollAnchor, TranscriptViewportCache, ui_elements};
use marlin_agent_ui::{UiComponentId, UiContent, UiTheme, UiTone};
use ratatui::{layout::Rect, style::Style};

pub(super) struct MainSlots<'a> {
    pub app: &'a App,
    pub theme: &'a UiTheme,
    pub cache: &'a mut TranscriptViewportCache,
    pub anchor: &'a mut ScrollAnchor,
    pub role_style: Style,
    pub static_content: Option<(&'a str, &'a [UiContent])>,
    pub focused_content: Option<(&'a str, &'a [UiContent])>,
    pub effective_scroll: usize,
}

impl ui_elements::UiSlotRenderer for MainSlots<'_> {
    fn title(&self, id: &UiComponentId) -> Option<&str> {
        match MainSource::from_id(id)? {
            MainSource::Static => self.static_content.map(|(title, _)| title),
            MainSource::Runtime => Some("Runtime policy"),
            MainSource::Extension => self.focused_content.map(|(title, _)| title),
            _ => None,
        }
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        id: &UiComponentId,
        field_tone: Option<UiTone>,
        skip: usize,
    ) {
        let Some(source) = MainSource::from_id(id) else {
            return;
        };
        let tone = field_tone.unwrap_or(UiTone::Normal);
        match source {
            MainSource::Static | MainSource::Runtime | MainSource::Extension => {
                let content = match source {
                    MainSource::Static => self.static_content.map(|(_, content)| content),
                    MainSource::Runtime => Some(self.app.runtime_content.as_slice()),
                    MainSource::Extension => self.focused_content.map(|(_, content)| content),
                    _ => None,
                };
                if let Some(content) = content {
                    let scroll = if matches!(source, MainSource::Static) {
                        self.app.static_buffer_scroll
                    } else {
                        0
                    };
                    ui_elements::render_content(
                        frame,
                        area,
                        content,
                        self.theme,
                        tone,
                        scroll.saturating_add(skip),
                    );
                }
            }
            MainSource::Review => {
                if let Some(prompt) = &self.app.tool_review_prompt {
                    self.cache.with_top_view(
                        &prompt.details,
                        area.width,
                        area.height,
                        usize::from(prompt.scroll).saturating_add(skip),
                        |viewport| {
                            viewport.render_with_role_style(
                                area,
                                frame.buffer_mut(),
                                self.role_style,
                            )
                        },
                    );
                }
            }
            MainSource::Messages => {
                let scroll = self.anchor.adjust(
                    &self.app.messages,
                    false,
                    area.width,
                    area.height,
                    self.app.message_scroll_up,
                );
                self.effective_scroll = self.cache.with_view(
                    &self.app.messages,
                    false,
                    area.width,
                    area.height,
                    scroll,
                    |viewport| {
                        viewport.render_with_role_style(area, frame.buffer_mut(), self.role_style);
                        viewport.scroll_up
                    },
                );
            }
            MainSource::Workspace => {
                if let Some(view) = self.app.workspace.view() {
                    self.effective_scroll = self.cache.with_top_view(
                        &view.transcript,
                        area.width,
                        area.height,
                        view.scroll_from_top.saturating_add(skip),
                        |viewport| {
                            viewport.render_with_role_style(
                                area,
                                frame.buffer_mut(),
                                self.role_style,
                            );
                            viewport.scroll_up
                        },
                    );
                }
            }
            MainSource::Conversation => {
                let pending = self.app.pending.is_some();
                let scroll = self.anchor.adjust(
                    &self.app.transcript,
                    pending,
                    area.width,
                    area.height,
                    self.app.scroll_up,
                );
                self.effective_scroll = self.cache.with_view(
                    &self.app.transcript,
                    pending,
                    area.width,
                    area.height,
                    scroll,
                    |viewport| {
                        viewport.render_with_role_style(area, frame.buffer_mut(), self.role_style);
                        viewport.scroll_up
                    },
                );
            }
        }
    }
}
