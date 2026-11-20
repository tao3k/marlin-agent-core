// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic Ratatui mapping for admitted Scheme UI elements.

mod core;
mod geometry;
mod widgets;

use core::terminal_text;
pub use core::{
    UiRenderInputs, UiSlotRenderer, render_element, render_prompt, render_template,
    render_with_facts, render_with_regions, render_with_scroll, render_with_slots, resolve_choice,
};
pub(crate) use core::{fits_area, fits_width, inline_text, tone_color};
pub use geometry::{
    Extent, HitRegion, RegionExtents, hit_test, hit_test_command, hit_test_scroll, layout_regions,
    layout_regions_with_facts, layout_regions_with_scroll, region_extent, resolve_size,
};
pub(crate) use widgets::{InputInsets, input_insets};
pub use widgets::{
    UiContentSlot, UiInputMark, UiInputValue, UiListData, UiListHit, UiListRow, UiListState,
    render_content, render_input, render_list,
};
