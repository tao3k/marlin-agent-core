// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui mapping for admitted Scheme UI elements.

mod core;
mod widgets;

use core::terminal_text;
pub(crate) use core::tone_color;
pub use core::{
    Extent, HitRegion, UiRenderInputs, UiSlotRenderer, hit_test, hit_test_scroll, layout_regions,
    layout_regions_with_facts, render_element, render_prompt, render_template, render_with_facts,
    render_with_regions, render_with_scroll, render_with_slots, resolve_choice,
};
pub use widgets::{UiContentSlot, UiListRow, render_content, render_input, render_list};
