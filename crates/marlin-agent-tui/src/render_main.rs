// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bind host-owned content and cached transcripts to declarative UI slots.

use super::{App, Composer, ScrollAnchor, TranscriptViewportCache, ui_elements};
use marlin_agent_ui::{
    UiCommandName, UiComponentId, UiContent, UiElement, UiListSource, UiSlotSource, UiWorkbench,
};
use ratatui::layout::Rect;

/// Bind host data to nodes while the common mapper traverses the full frame.
pub(super) struct FrameSlots<'a> {
    pub main: MainSlots<'a>,
    pub editor: EditorSlots<'a>,
    pub app: &'a App,
    pub ui: &'a UiWorkbench,
    pub suggestions: &'a [UiCommandName],
    pub extensions: &'a [(&'a UiComponentId, &'a str)],
    pub selected: Option<usize>,
    pub menu_prefix: Option<&'a str>,
}

impl ui_elements::UiSlotRenderer for FrameSlots<'_> {
    fn list_state(&self) -> Option<&std::cell::RefCell<ui_elements::UiListState>> {
        Some(&self.app.list_state)
    }

    fn tree(&self, source: &UiSlotSource) -> Option<(&UiElement, usize)> {
        self.main.tree(source)
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        source: &UiSlotSource,
        field_template: Option<&UiElement>,
        inputs: &ui_elements::UiRenderInputs<'_>,
        skip: usize,
    ) {
        if *source == UiSlotSource::TransientHint {
            if let Some(hint) = &self.app.transient_hint {
                ui_elements::render_with_facts(frame, area, hint, &mut Vec::new(), inputs);
            }
            return;
        }
        self.main
            .render(frame, area, source, field_template, inputs, skip);
    }

    fn input(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        element: &UiElement,
        inputs: &ui_elements::UiRenderInputs<'_>,
    ) {
        self.editor.input(frame, area, element, inputs);
    }

    fn list(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        element: &UiElement,
        inputs: &ui_elements::UiRenderInputs<'_>,
        skip: usize,
    ) {
        let UiElement::List { source, .. } = element else {
            return;
        };
        if !matches!(source, UiListSource::Candidates | UiListSource::Extensions) {
            self.main.list(frame, area, element, inputs, skip);
            return;
        }
        if *source == UiListSource::Candidates {
            let count = self
                .app
                .tool_review_prompt
                .as_ref()
                .map_or(self.suggestions.len(), |prompt| prompt.menu.choices.len());
            ui_elements::render_list(
                frame,
                area,
                element,
                &self.ui.theme,
                ui_elements::UiListData {
                    count,
                    selected: self.selected,
                    extent: inputs.extent,
                    facts: inputs.facts,
                    bindings: inputs.bindings,
                },
                &mut self.app.list_state.borrow_mut(),
                |index| {
                    let (key, bindings) = if let Some(prompt) = &self.app.tool_review_prompt {
                        let choice = prompt.menu.choices.get(index)?;
                        (
                            Some(UiComponentId::new(choice.decision.selection_id()).ok()?),
                            vec![
                                ("name", choice.label.as_str().into()),
                                ("description", choice.description.as_str().into()),
                            ],
                        )
                    } else {
                        let name = self.suggestions.get(index)?;
                        let command = self
                            .ui
                            .commands()
                            .iter()
                            .find(|command| &command.name == name)?;
                        (
                            Some(UiComponentId::new(name.as_str()).ok()?),
                            vec![
                                (
                                    "name",
                                    super::menu_display_name(
                                        command.name.as_str(),
                                        self.menu_prefix,
                                    )
                                    .into(),
                                ),
                                (
                                    "description",
                                    super::header_value(&command.description).into(),
                                ),
                            ],
                        )
                    };
                    Some(ui_elements::UiListRow { key, bindings })
                },
            );
        } else {
            let selected = self
                .extensions
                .iter()
                .position(|(id, _)| *id == &self.ui.focus);
            ui_elements::render_list(
                frame,
                area,
                element,
                &self.ui.theme,
                ui_elements::UiListData {
                    count: self.extensions.len(),
                    selected,
                    extent: inputs.extent,
                    facts: inputs.facts,
                    bindings: inputs.bindings,
                },
                &mut self.app.list_state.borrow_mut(),
                |index| {
                    Some(ui_elements::UiListRow {
                        key: None,
                        bindings: vec![(
                            "label",
                            super::header_value(self.extensions.get(index)?.1).into(),
                        )],
                    })
                },
            );
        }
    }
}

pub(super) struct EditorSlots<'a> {
    pub composer: &'a Composer,
    pub expansion_bindings: &'a [marlin_agent_ui::UiKeyBinding],
    pub show_cursor: bool,
    pub read_only: bool,
}

impl ui_elements::UiSlotRenderer for EditorSlots<'_> {
    fn render(
        &mut self,
        _: &mut ratatui::Frame<'_>,
        _: Rect,
        _: &UiSlotSource,
        _: Option<&UiElement>,
        _: &ui_elements::UiRenderInputs<'_>,
        _: usize,
    ) {
    }

    fn input(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        element: &UiElement,
        inputs: &ui_elements::UiRenderInputs<'_>,
    ) {
        ui_elements::render_input(
            frame,
            area,
            element,
            inputs,
            self.show_cursor,
            |width, height| {
                if self.read_only {
                    None
                } else if self.expansion_bindings.is_empty() {
                    self.composer.view(width, height)
                } else {
                    self.composer
                        .view_with_expansion(width, height, self.expansion_bindings)
                }
            },
        );
    }
}

pub(super) struct MainSlots<'a> {
    pub app: &'a App,
    pub cache: &'a mut TranscriptViewportCache,
    pub anchor: &'a mut ScrollAnchor,
    pub static_content: Option<&'a UiElement>,
    pub focused_content: Option<&'a UiElement>,
    pub dynamic_content: Option<&'a [UiContent]>,
    pub effective_scroll: usize,
}

impl ui_elements::UiSlotRenderer for MainSlots<'_> {
    fn tree(&self, source: &UiSlotSource) -> Option<(&UiElement, usize)> {
        match source {
            UiSlotSource::SelectedBuffer if self.dynamic_content.is_none() => self
                .static_content
                .map(|tree| (tree, self.app.static_buffer_scroll)),
            UiSlotSource::FocusedExtension => self.focused_content.map(|tree| (tree, 0)),
            _ => None,
        }
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        source: &UiSlotSource,
        field_template: Option<&UiElement>,
        inputs: &ui_elements::UiRenderInputs<'_>,
        skip: usize,
    ) {
        let content = match source {
            UiSlotSource::SelectedBuffer => self.dynamic_content,
            UiSlotSource::RuntimePolicy => Some(self.app.runtime_content.as_slice()),
            UiSlotSource::FocusedExtension | UiSlotSource::TransientHint => None,
        };
        if let (Some(content), Some(field_template)) = (content, field_template) {
            let scroll = if *source == UiSlotSource::SelectedBuffer {
                self.app.static_buffer_scroll
            } else {
                0
            };
            ui_elements::render_content(
                frame,
                area,
                content,
                field_template,
                inputs,
                scroll.saturating_add(skip),
            );
        }
    }

    fn list(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        element: &UiElement,
        inputs: &ui_elements::UiRenderInputs<'_>,
        skip: usize,
    ) {
        let UiElement::List { source, .. } = element else {
            return;
        };
        let app = self.app;
        let paint = |frame: &mut ratatui::Frame<'_>,
                     viewport: &super::super::viewport::TranscriptViewport| {
            ui_elements::render_list(
                frame,
                area,
                element,
                inputs.theme,
                ui_elements::UiListData {
                    count: viewport.lines.len(),
                    selected: None,
                    extent: inputs.extent,
                    facts: inputs.facts,
                    bindings: inputs.bindings,
                },
                &mut app.list_state.borrow_mut(),
                |index| {
                    let (role, body) = viewport.lines.get(index)?.fields();
                    Some(ui_elements::UiListRow {
                        key: None,
                        bindings: vec![("role-text", role.into()), ("body-text", body.into())],
                    })
                },
            );
        };
        match source {
            UiListSource::Review => {
                if let Some(prompt) = &self.app.tool_review_prompt {
                    self.cache.with_top_view(
                        &prompt.details,
                        area.width,
                        area.height,
                        usize::from(prompt.scroll).saturating_add(skip),
                        |viewport| paint(frame, viewport),
                    );
                }
            }
            UiListSource::Messages => {
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
                        paint(frame, viewport);
                        viewport.scroll_up
                    },
                );
            }
            UiListSource::Workspace => {
                if let Some(view) = self.app.workspace.view() {
                    self.effective_scroll = self.cache.with_top_view(
                        &view.transcript,
                        area.width,
                        area.height,
                        view.scroll_from_top.saturating_add(skip),
                        |viewport| {
                            paint(frame, viewport);
                            viewport.scroll_up
                        },
                    );
                }
            }
            UiListSource::Conversation => {
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
                        paint(frame, viewport);
                        viewport.scroll_up
                    },
                );
            }
            _ => {}
        }
    }
}
