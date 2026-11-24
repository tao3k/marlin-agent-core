// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Host-neutral UI primitives admitted from rooted Scheme values.

mod decode;
mod tree;

pub use decode::decode_element;
pub(crate) use decode::{decode_tone, inline_template};
pub use tree::{
    UiBackground, UiBarSegment, UiBarSide, UiElement, UiInputAdornment, UiListSource, UiListValue,
    UiSize, UiSlotSource, UiSpan, UiTemplateSpan, UiTemplateValue, UiTone,
};
