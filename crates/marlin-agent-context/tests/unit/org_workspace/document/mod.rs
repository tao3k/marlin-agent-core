mod basic;
mod contracts;

fn slice<'a>(text: &'a str, span: &marlin_context_org_model::OrgSourceSpan) -> &'a str {
    &text[span.start_byte..span.end_byte]
}
