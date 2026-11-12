// SPDX-FileCopyrightText: 2026 Tao 3000
// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod references;
mod standard_library;

fn slice<'a>(text: &'a str, span: &marlin_context_org_model::OrgSourceSpan) -> &'a str {
    &text[span.start_byte..span.end_byte]
}

pub(super) fn cases() -> impl Iterator<Item = (&'static str, fn())> {
    references::CASES
        .iter()
        .chain(standard_library::CASES)
        .copied()
}
