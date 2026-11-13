// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Layout only the visible transcript rows, independently of terminal scroll limits.

use std::{
    borrow::Cow,
    collections::VecDeque,
    mem::size_of,
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

use ratatui::{
    buffer::{Buffer, CellWidth},
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::wrap::visit_rows;

/// Transcript mutations advance one revision; readers cannot mutate entries
/// without invalidating a cached viewport.
pub(crate) struct Transcript {
    entries: Vec<(String, String)>,
    /// Thinking entries are appended in transcript order; one sorted slot
    /// vector supports ordinal selection and indexed lookup without a scan.
    folds: Vec<(usize, ThinkingFold)>,
    identity: u64,
    revision: u64,
    non_tail_revision: u64,
}

/// The hidden side of a thinking row is kept out of viewport layout/cache.
struct ThinkingFold {
    other: String,
    other_role: String,
    collapsed: bool,
}

static NEXT_TRANSCRIPT_ID: AtomicU64 = AtomicU64::new(1);

impl Transcript {
    pub(crate) fn new(entries: Vec<(String, String)>) -> Self {
        let identity = NEXT_TRANSCRIPT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("transcript identity exhausted");
        Self {
            folds: Vec::new(),
            entries,
            identity,
            revision: 0,
            non_tail_revision: 0,
        }
    }

    fn changed(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("transcript revision exhausted");
    }

    pub(crate) fn push(&mut self, entry: (String, String)) {
        self.changed();
        self.entries.push(entry);
    }

    pub(crate) fn push_thinking(
        &mut self,
        expanded_role: String,
        collapsed_role: &str,
        content: String,
        collapsed: bool,
    ) {
        self.changed();
        let (role, other_role, visible, other) = if collapsed {
            (
                collapsed_role.to_owned(),
                expanded_role,
                String::new(),
                content,
            )
        } else {
            (
                expanded_role,
                collapsed_role.to_owned(),
                content,
                String::new(),
            )
        };
        debug_assert!(
            self.folds
                .last()
                .is_none_or(|(index, _)| *index < self.entries.len())
        );
        self.folds.push((
            self.entries.len(),
            ThinkingFold {
                other,
                other_role,
                collapsed,
            },
        ));
        self.entries.push((role, visible));
    }

    pub(crate) fn content(&self, index: usize) -> &str {
        match self.fold_slot(index) {
            Some((_, fold)) if fold.collapsed => &fold.other,
            _ => &self.entries[index].1,
        }
    }

    fn fold_slot(&self, index: usize) -> Option<&(usize, ThinkingFold)> {
        self.folds
            .binary_search_by_key(&index, |(entry, _)| *entry)
            .ok()
            .map(|slot| &self.folds[slot])
    }

    pub(crate) fn toggle_thinking_from_end(&mut self, ordinal: usize) -> bool {
        if ordinal == 0 || ordinal > self.folds.len() {
            return false;
        }
        let slot = self.folds.len() - ordinal;
        let (index, fold) = &mut self.folds[slot];
        let index = *index;
        std::mem::swap(&mut self.entries[index].0, &mut fold.other_role);
        std::mem::swap(&mut self.entries[index].1, &mut fold.other);
        fold.collapsed = !fold.collapsed;
        self.changed();
        self.non_tail_revision = self
            .non_tail_revision
            .checked_add(1)
            .expect("transcript mutation counter exhausted");
        true
    }

    pub(crate) fn clear(&mut self) {
        self.changed();
        self.non_tail_revision = self
            .non_tail_revision
            .checked_add(1)
            .expect("transcript mutation counter exhausted");
        self.entries.clear();
        self.folds.clear();
    }

    pub(crate) fn retain_latest(&mut self, limit: usize) {
        let remove = self.entries.len().saturating_sub(limit);
        if remove > 0 {
            self.changed();
            self.non_tail_revision = self
                .non_tail_revision
                .checked_add(1)
                .expect("transcript mutation counter exhausted");
            self.entries.drain(..remove);
            let first = self.folds.partition_point(|(index, _)| *index < remove);
            self.folds.drain(..first);
            for (index, _) in &mut self.folds {
                *index -= remove;
            }
        }
    }

    pub(crate) fn append_content(&mut self, index: usize, content: &str) {
        if let Ok(slot) = self.folds.binary_search_by_key(&index, |(entry, _)| *entry) {
            let fold = &mut self.folds[slot].1;
            if fold.collapsed {
                fold.other.push_str(content);
                return;
            }
        }
        self.changed();
        if index + 1 != self.entries.len() {
            self.non_tail_revision = self
                .non_tail_revision
                .checked_add(1)
                .expect("transcript mutation counter exhausted");
        }
        self.entries[index].1.push_str(content);
    }
}

struct TailSnapshot {
    identity: u64,
    non_tail_revision: u64,
    revision: u64,
    width: u16,
    height: u16,
    entries: usize,
    last_rows: usize,
    pending_rows: usize,
    layout: TailLayout,
}

/// Completed source lines cannot reflow when content is appended after a newline.
struct TailLayout {
    prefix_end: usize,
    prefix_rows: usize,
    role_rows: usize,
}

/// Keeps a scrolled viewport at the same rows when only the transcript tail changes.
#[derive(Default)]
pub(crate) struct ScrollAnchor {
    snapshot: Option<TailSnapshot>,
}

fn wrapped_rows(text: &str, width: u16) -> usize {
    let mut rows = 0usize;
    visit_rows(text, width, |_| {
        rows += 1;
        true
    });
    rows
}

impl TailLayout {
    fn new(entry: &(String, String), width: u16) -> Self {
        Self {
            prefix_end: 0,
            prefix_rows: 0,
            role_rows: if entry.0.is_empty() {
                0
            } else {
                wrapped_rows(&entry.0, width)
            },
        }
    }

    fn rows(&mut self, entry: &(String, String), width: u16) -> usize {
        let mut tail_rows = 0;
        for line in entry.1[self.prefix_end..].split_inclusive('\n') {
            if let Some(complete) = line.strip_suffix('\n') {
                self.prefix_rows += wrapped_rows(complete, width);
                self.prefix_end += line.len();
            } else {
                tail_rows = wrapped_rows(line, width);
            }
        }
        self.role_rows + self.prefix_rows + tail_rows + wrapped_rows("", width)
    }
}

impl ScrollAnchor {
    pub(crate) fn adjust(
        &mut self,
        transcript: &Transcript,
        pending: bool,
        width: u16,
        height: u16,
        scroll: usize,
    ) -> usize {
        if scroll == 0 || width == 0 || height == 0 {
            self.snapshot = None;
            return scroll;
        }
        let pending_rows = if pending {
            wrapped_rows("Waiting for gateway…", width)
        } else {
            0
        };
        let Some(last) = transcript.last() else {
            self.snapshot = None;
            return scroll;
        };
        let previous = self.snapshot.take();
        if let Some(snapshot) = previous.as_ref()
            && snapshot.identity == transcript.identity
            && snapshot.non_tail_revision == transcript.non_tail_revision
            && snapshot.revision == transcript.revision
            && snapshot.width == width
            && snapshot.height == height
            && snapshot.pending_rows == pending_rows
        {
            self.snapshot = previous;
            return scroll;
        }
        let mut adjusted = scroll;
        let (layout, last_rows) = if let Some(mut snapshot) = previous
            && snapshot.identity == transcript.identity
            && snapshot.non_tail_revision == transcript.non_tail_revision
            && snapshot.width == width
            && snapshot.height == height
            && snapshot.entries <= transcript.len()
        {
            let first = snapshot.entries - 1;
            let mut new_rows = snapshot.layout.rows(&transcript[first], width) + pending_rows;
            let mut layout = snapshot.layout;
            let mut last_rows = new_rows - pending_rows;
            for entry in &transcript[snapshot.entries..] {
                layout = TailLayout::new(entry, width);
                last_rows = layout.rows(entry, width);
                new_rows += last_rows;
            }
            let old_rows = snapshot.last_rows + snapshot.pending_rows;
            adjusted = scroll
                .saturating_add(new_rows.saturating_sub(old_rows))
                .saturating_sub(old_rows.saturating_sub(new_rows));
            (layout, last_rows)
        } else {
            let mut layout = TailLayout::new(last, width);
            let last_rows = layout.rows(last, width);
            (layout, last_rows)
        };
        self.snapshot = Some(TailSnapshot {
            identity: transcript.identity,
            non_tail_revision: transcript.non_tail_revision,
            revision: transcript.revision,
            width,
            height,
            entries: transcript.len(),
            last_rows,
            pending_rows,
            layout,
        });
        adjusted
    }
}

impl Deref for Transcript {
    type Target = [(String, String)];
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

#[derive(Eq, PartialEq)]
struct ViewportKey {
    identity: u64,
    revision: u64,
    pending: bool,
    width: u16,
    height: u16,
    scroll_up: usize,
    from_top: bool,
}

const VIEWPORT_CACHE_BYTES: usize = 1024 * 1024;
const TAIL_ROW_PARTS_LIMIT: usize = 16_384;
const ROW_CHECKPOINT_STRIDE: usize = 128;
const ROW_CHECKPOINT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
struct RowCheckpoint {
    entry: usize,
    rows: usize,
}

/// Width-specific row positions for deep scrolling. Only every 128th entry is
/// retained; the source transcript remains the authority for rendered text.
struct RowCheckpointIndex {
    identity: u64,
    non_tail_revision: u64,
    revision: u64,
    width: u16,
    indexed_entries: usize,
    total_rows: usize,
    tail_rows: usize,
    tail_layout: Option<TailLayout>,
    checkpoints: Vec<RowCheckpoint>,
}

fn fits_one_row(text: &str, width: u16) -> bool {
    if text.len() <= usize::from(width) && text.bytes().all(|byte| !byte.is_ascii_control()) {
        // A UTF-8 grapheme cannot occupy more terminal cells than its source
        // bytes; ASCII controls are excluded because layout sanitizes them.
        return true;
    }
    // CellWidth is the same terminal width operation used by the row visitor.
    // Bound the input so its u16 result cannot wrap on a pathological string.
    text.len() <= 1024 && !text.chars().any(char::is_control) && text.cell_width() <= width
}

fn entry_rows(entry: &(String, String), width: u16) -> usize {
    // Most completed fragments fit one row. Check without allocating the
    // word-wrapping segments, including short Unicode labels and messages.
    if !entry.1.is_empty() && fits_one_row(&entry.0, width) && fits_one_row(&entry.1, width) {
        return 2 + usize::from(!entry.0.is_empty());
    }
    (if entry.0.is_empty() {
        0
    } else {
        wrapped_rows(&entry.0, width)
    }) + entry
        .1
        .split_terminator('\n')
        .map(|line| wrapped_rows(line, width))
        .sum::<usize>()
        + wrapped_rows("", width)
}

impl RowCheckpointIndex {
    fn update(&mut self, transcript: &Transcript) -> bool {
        if self.identity != transcript.identity
            || self.non_tail_revision != transcript.non_tail_revision
            || self.width == 0
            || transcript.len() < self.indexed_entries
        {
            return false;
        }
        if self.revision == transcript.revision {
            return true;
        }
        if let Some(layout) = self.tail_layout.as_mut() {
            let rows = layout.rows(&transcript[self.indexed_entries - 1], self.width);
            self.total_rows = self.total_rows - self.tail_rows + rows;
            self.tail_rows = rows;
        }
        for index in self.indexed_entries..transcript.len() {
            if index % ROW_CHECKPOINT_STRIDE == 0 {
                if (self.checkpoints.len() + 1).saturating_mul(size_of::<RowCheckpoint>())
                    > ROW_CHECKPOINT_BYTES
                {
                    return false;
                }
                self.checkpoints.push(RowCheckpoint {
                    entry: index,
                    rows: self.total_rows,
                });
            }
            let last = index + 1 == transcript.len();
            let (rows, layout) = if last {
                let mut layout = TailLayout::new(&transcript[index], self.width);
                (layout.rows(&transcript[index], self.width), Some(layout))
            } else {
                (entry_rows(&transcript[index], self.width), None)
            };
            self.total_rows = self.total_rows.saturating_add(rows);
            self.tail_rows = rows;
            self.tail_layout = layout;
        }
        self.indexed_entries = transcript.len();
        self.revision = transcript.revision;
        true
    }

    fn build(transcript: &Transcript, width: u16) -> Option<Self> {
        let mut index = Self {
            identity: transcript.identity,
            non_tail_revision: transcript.non_tail_revision,
            revision: u64::MAX,
            width,
            indexed_entries: 0,
            total_rows: 0,
            tail_rows: 0,
            tail_layout: None,
            checkpoints: Vec::new(),
        };
        index.update(transcript).then_some(index)
    }

    fn viewport(
        &self,
        transcript: &Transcript,
        pending: bool,
        width: u16,
        height: u16,
        scroll_up: usize,
    ) -> TranscriptViewport {
        let total = self
            .total_rows
            .saturating_add(usize::from(pending) * wrapped_rows("Waiting for gateway…", width));
        let max_scroll = total.saturating_sub(usize::from(height));
        let scroll = scroll_up.min(max_scroll);
        let target = max_scroll - scroll;
        let checkpoint = self
            .checkpoints
            .partition_point(|point| point.rows <= target);
        let point = self.checkpoints[checkpoint.saturating_sub(1)];
        let mut view = transcript_from_top(
            &transcript[point.entry..],
            pending,
            width,
            height,
            target - point.rows,
        );
        view.scroll_up = scroll;
        view
    }
}

/// Retains at most one viewport within its accounted allocation budget.
pub(crate) struct TranscriptViewportCache {
    cached: Option<(ViewportKey, TranscriptViewport)>,
    row_index: Option<RowCheckpointIndex>,
    max_bytes: usize,
    #[cfg(test)]
    builds: usize,
}

impl Default for TranscriptViewportCache {
    fn default() -> Self {
        Self {
            cached: None,
            row_index: None,
            max_bytes: VIEWPORT_CACHE_BYTES,
            #[cfg(test)]
            builds: 0,
        }
    }
}

impl TranscriptViewportCache {
    pub(crate) fn with_view<R>(
        &mut self,
        transcript: &Transcript,
        pending: bool,
        width: u16,
        height: u16,
        scroll_up: usize,
        use_view: impl FnOnce(&TranscriptViewport) -> R,
    ) -> R {
        let key = ViewportKey {
            identity: transcript.identity,
            revision: transcript.revision,
            pending,
            width,
            height,
            scroll_up,
            from_top: false,
        };
        self.with_key(transcript, key, use_view)
    }

    pub(crate) fn with_top_view<R>(
        &mut self,
        transcript: &Transcript,
        width: u16,
        height: u16,
        scroll_from_top: usize,
        use_view: impl FnOnce(&TranscriptViewport) -> R,
    ) -> R {
        let key = ViewportKey {
            identity: transcript.identity,
            revision: transcript.revision,
            pending: false,
            width,
            height,
            scroll_up: scroll_from_top,
            from_top: true,
        };
        self.with_key(transcript, key, use_view)
    }

    fn with_key<R>(
        &mut self,
        transcript: &Transcript,
        key: ViewportKey,
        use_view: impl FnOnce(&TranscriptViewport) -> R,
    ) -> R {
        if self.cached.as_ref().is_some_and(|(old, _)| *old == key) {
            return use_view(&self.cached.as_ref().expect("cached viewport").1);
        }
        self.cached = None;
        let view = if key.from_top {
            transcript_from_top(
                transcript,
                key.pending,
                key.width,
                key.height,
                key.scroll_up,
            )
        } else if key.scroll_up >= 256
            && transcript.len() >= ROW_CHECKPOINT_STRIDE * 2
            && transcript
                .len()
                .div_ceil(ROW_CHECKPOINT_STRIDE)
                .saturating_mul(size_of::<RowCheckpoint>())
                <= ROW_CHECKPOINT_BYTES
        {
            if !self
                .row_index
                .as_mut()
                .is_some_and(|index| index.width == key.width && index.update(transcript))
            {
                self.row_index = RowCheckpointIndex::build(transcript, key.width);
            }
            self.row_index.as_ref().map_or_else(
                || {
                    transcript_viewport(
                        transcript,
                        key.pending,
                        key.width,
                        key.height,
                        key.scroll_up,
                    )
                },
                |index| {
                    index.viewport(
                        transcript,
                        key.pending,
                        key.width,
                        key.height,
                        key.scroll_up,
                    )
                },
            )
        } else {
            transcript_viewport(
                transcript,
                key.pending,
                key.width,
                key.height,
                key.scroll_up,
            )
        };
        #[cfg(test)]
        {
            self.builds += 1;
        }
        if view
            .accounted_bytes()
            .saturating_add(size_of::<ViewportKey>())
            <= self.max_bytes
        {
            self.cached = Some((key, view));
            use_view(&self.cached.as_ref().expect("cached viewport").1)
        } else {
            use_view(&view)
        }
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "used by the external unit-test harness")]
    pub(crate) fn build_count(&self) -> usize {
        self.builds
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "used by the external unit-test harness")]
    pub(crate) fn with_budget(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            ..Self::default()
        }
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "used by the external unit-test benchmark")]
    pub(crate) fn row_index_bytes(&self) -> usize {
        self.row_index.as_ref().map_or(0, |index| {
            index
                .checkpoints
                .capacity()
                .saturating_mul(size_of::<RowCheckpoint>())
        })
    }
}

pub(crate) struct TranscriptViewport {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) scroll_up: usize,
}

impl TranscriptViewport {
    fn accounted_bytes(&self) -> usize {
        let mut bytes = self
            .lines
            .capacity()
            .saturating_mul(size_of::<Line<'static>>());
        for line in &self.lines {
            bytes = bytes.saturating_add(
                line.spans
                    .capacity()
                    .saturating_mul(size_of::<Span<'static>>()),
            );
            for span in &line.spans {
                if let Cow::Owned(text) = &span.content {
                    bytes = bytes.saturating_add(text.capacity());
                }
            }
        }
        bytes
    }

    pub(crate) fn render(&self, area: Rect, buffer: &mut Buffer) {
        let area = area.intersection(buffer.area);
        for (row, line) in self.lines.iter().take(usize::from(area.height)).enumerate() {
            let mut x = 0u16;
            for grapheme in line.styled_graphemes(Style::default()) {
                let width = grapheme.symbol.cell_width();
                if width == 0 {
                    continue;
                }
                if width > area.width.saturating_sub(x) {
                    break;
                }
                buffer[(area.x + x, area.y + row as u16)]
                    .set_symbol(grapheme.symbol)
                    .set_style(grapheme.style);
                x += width;
            }
        }
    }
}

struct ReverseWindow {
    width: u16,
    needed: usize,
    skip: usize,
    lines: Vec<Line<'static>>,
    fast_tail: bool,
}

impl ReverseWindow {
    fn add(&mut self, text: &str, style: Style) -> bool {
        // Streaming answers often contain a very long line. At the live tail,
        // keep only the visible row slices while scanning once. Deep scrolling
        // retains the two-pass path so memory does not grow with scroll depth.
        if self.fast_tail
            && self.skip == 0
            && self.needed > 0
            && self.needed <= 1024
            && self.width <= 1024
            && text.len() >= 4096
        {
            let mut rows: VecDeque<Vec<&str>> = VecDeque::new();
            let mut retained_parts = 0usize;
            let mut exceeded = false;
            visit_rows(text, self.width, |row| {
                let mut slot = if rows.len() == self.needed {
                    rows.pop_front().expect("full viewport")
                } else {
                    Vec::new()
                };
                retained_parts -= slot.capacity();
                if retained_parts.saturating_add(slot.capacity().max(row.len()))
                    > TAIL_ROW_PARTS_LIMIT
                {
                    exceeded = true;
                    return false;
                }
                slot.clear();
                slot.extend_from_slice(row);
                retained_parts = retained_parts.saturating_add(slot.capacity());
                if retained_parts > TAIL_ROW_PARTS_LIMIT {
                    exceeded = true;
                    return false;
                }
                rows.push_back(slot);
                true
            });
            if !exceeded {
                self.needed -= rows.len();
                self.lines.extend(
                    rows.into_iter()
                        .rev()
                        .map(|row| Line::styled(row.concat(), style)),
                );
                return self.needed == 0;
            }
        }
        let mut count = 0usize;
        visit_rows(text, self.width, |_| {
            count += 1;
            true
        });
        if self.skip >= count {
            self.skip -= count;
            return false;
        }
        let end = count - self.skip;
        let start = end.saturating_sub(self.needed);
        let mut selected = Vec::new();
        let mut index = 0usize;
        visit_rows(text, self.width, |row| {
            if index >= start && index < end {
                selected.push(Line::styled(row.concat(), style));
            }
            index += 1;
            index < end
        });
        self.needed -= selected.len();
        self.skip = 0;
        selected.reverse();
        self.lines.extend(selected);
        self.needed == 0
    }
}

/// Materialize at most the visible rows, including for deep scroll and long lines.
pub(crate) fn transcript_viewport(
    transcript: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
    scroll_up: usize,
) -> TranscriptViewport {
    transcript_viewport_impl(transcript, pending, width, height, scroll_up, true)
}

#[cfg(test)]
#[allow(dead_code, reason = "used by the external unit-test harness")]
pub(crate) fn transcript_viewport_reference(
    transcript: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
    scroll_up: usize,
) -> TranscriptViewport {
    transcript_viewport_impl(transcript, pending, width, height, scroll_up, false)
}

fn transcript_viewport_impl(
    transcript: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
    scroll_up: usize,
    fast_tail: bool,
) -> TranscriptViewport {
    if width == 0 || height == 0 {
        return TranscriptViewport {
            lines: Vec::new(),
            scroll_up,
        };
    }
    let mut window = ReverseWindow {
        width,
        needed: usize::from(height),
        skip: scroll_up,
        lines: Vec::new(),
        fast_tail,
    };
    let role_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    if !pending || !window.add("Waiting for gateway…", Style::default()) {
        'entries: for (role, content) in transcript.iter().rev() {
            if window.add("", Style::default()) {
                break;
            }
            for line in content.split_terminator('\n').rev() {
                if window.add(line, Style::default()) {
                    break 'entries;
                }
            }
            if !role.is_empty() && window.add(role, role_style) {
                break;
            }
        }
    }
    if window.needed > 0 && scroll_up > 0 {
        let effective_scroll = scroll_up.saturating_sub(window.skip.saturating_add(window.needed));
        let mut top = transcript_top(transcript, pending, width, height);
        top.scroll_up = effective_scroll;
        return top;
    }
    window.lines.reverse();
    TranscriptViewport {
        lines: window.lines,
        scroll_up,
    }
}

fn transcript_top(
    transcript: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
) -> TranscriptViewport {
    transcript_from_top(transcript, pending, width, height, 0)
}

/// Project from the beginning, stopping after the requested visible window.
pub(crate) fn transcript_from_top(
    transcript: &[(String, String)],
    pending: bool,
    width: u16,
    height: u16,
    scroll_from_top: usize,
) -> TranscriptViewport {
    if width == 0 || height == 0 {
        return TranscriptViewport {
            lines: Vec::new(),
            scroll_up: scroll_from_top,
        };
    }
    let mut lines = Vec::new();
    let mut seen = 0usize;
    let end = scroll_from_top.saturating_add(usize::from(height));
    let mut add = |text: &str, style: Style| {
        visit_rows(text, width, |row| {
            if seen >= scroll_from_top {
                lines.push(Line::styled(row.concat(), style));
            }
            seen += 1;
            seen < end
        });
        seen >= end
    };
    let role_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut filled = false;
    'entries: for (role, content) in transcript {
        if !role.is_empty() && add(role, role_style) {
            filled = true;
            break;
        }
        for line in content.split_terminator('\n') {
            if add(line, Style::default()) {
                filled = true;
                break 'entries;
            }
        }
        if add("", Style::default()) {
            filled = true;
            break;
        }
    }
    if pending && !filled {
        add("Waiting for gateway…", Style::default());
    }
    let max_scroll = seen.saturating_sub(usize::from(height));
    if !filled && scroll_from_top > max_scroll {
        let mut bottom = transcript_viewport(transcript, pending, width, height, 0);
        bottom.scroll_up = max_scroll;
        return bottom;
    }
    TranscriptViewport {
        lines,
        scroll_up: scroll_from_top,
    }
}
