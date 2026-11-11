// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

mod basic;
mod contracts;

fn slice<'a>(text: &'a str, span: &marlin_context_org_model::OrgSourceSpan) -> &'a str {
    &text[span.start_byte..span.end_byte]
}
