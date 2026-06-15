mod references;
mod standard_library;

fn slice<'a>(text: &'a str, span: &marlin_org_model::OrgSourceSpan) -> &'a str {
    &text[span.start_byte..span.end_byte]
}
