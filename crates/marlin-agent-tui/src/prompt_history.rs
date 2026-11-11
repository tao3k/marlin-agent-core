// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Navigation over completed conversation prompts while preserving the draft.

use super::composer::{Composer, MAX_DRAFT_BYTES};
use marlin_agent_protocol::{ModelGatewayMessage, ModelGatewayMessageRole};

#[derive(Default)]
pub(crate) struct PromptHistory {
    draft: Option<Composer>,
    index: Option<usize>,
}

impl PromptHistory {
    pub(crate) fn previous(&mut self, composer: &mut Composer, history: &[ModelGatewayMessage]) {
        let end = self.index.unwrap_or(history.len());
        let Some(index) = (0..end).rev().find(|&i| eligible(&history[i])) else {
            return;
        };
        if self.draft.is_none() {
            self.draft = Some(std::mem::take(composer));
        }
        *composer = from_prompt(&history[index].content);
        self.index = Some(index);
    }

    pub(crate) fn next(&mut self, composer: &mut Composer, history: &[ModelGatewayMessage]) {
        let Some(index) = self.index else {
            return;
        };
        if let Some(next) = (index + 1..history.len()).find(|&i| eligible(&history[i])) {
            *composer = from_prompt(&history[next].content);
            self.index = Some(next);
        } else {
            self.restore(composer);
        }
    }

    pub(crate) fn restore(&mut self, composer: &mut Composer) {
        if let Some(draft) = self.draft.take() {
            *composer = draft;
        }
        self.index = None;
    }
}

fn eligible(message: &ModelGatewayMessage) -> bool {
    message.role == ModelGatewayMessageRole::User && message.content.len() <= MAX_DRAFT_BYTES
}

fn from_prompt(text: &str) -> Composer {
    let mut composer = Composer::default();
    composer.insert_text(text);
    composer
}
