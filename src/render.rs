//! Compiling Typst sources to HTML, with diagnostics and metadata extraction.

use std::time::Instant;

use ecow::EcoVec;
use typst::diag::SourceDiagnostic;
use typst::foundations::{Dict, Label, Selector, Value};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_html::{HtmlDocument, HtmlOptions};
use typst_kit::diagnostics::termcolor::{ColorChoice, StandardStream};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};

use crate::Res;
use crate::model::FrontMatter;
use crate::world::{Shared, SiteWorld};

/// Compile a page (index, tag listing, or standalone page) to HTML.
pub(crate) fn render_page(shared: &Shared, main_rel: &str, inputs: Option<Dict>) -> Res<String> {
    let (html, _doc) = compile(shared, main_rel, inputs)?;
    Ok(html)
}

/// Compile a post to HTML and pull its required `<frontmatter>` metadata.
pub(crate) fn render_post(shared: &Shared, main_rel: &str) -> Res<(String, FrontMatter)> {
    let (html, doc) = compile(shared, main_rel, None)?;
    let frontmatter = query_frontmatter(&doc)
        .map(serde_json::from_value)
        .transpose()?
        .ok_or_else(|| format!("{main_rel}: missing <frontmatter> metadata"))?;
    Ok((html, frontmatter))
}

/// Compile `main_rel` to HTML (stylesheet injected), returning the document too
/// so callers can introspect it.
fn compile(shared: &Shared, main_rel: &str, inputs: Option<Dict>) -> Res<(String, HtmlDocument)> {
    let world = SiteWorld::new(shared, main_rel, inputs)?;
    let started = Instant::now();

    let compiled = typst::compile::<HtmlDocument>(&world);
    report_warnings(&world, &compiled.warnings);
    let doc = match compiled.output {
        Ok(doc) => doc,
        Err(diags) => {
            emit(&world, &diags);
            return Err(format!("failed to compile {main_rel}").into());
        }
    };

    let html = match typst_html::html(&doc, &HtmlOptions { pretty: true }) {
        Ok(html) => html,
        Err(diags) => {
            emit(&world, &diags);
            return Err(format!("failed to render {main_rel} to HTML").into());
        }
    };
    let html = inject_css(&html);

    tracing::debug!(
        "compiled {main_rel} in {} ms",
        started.elapsed().as_millis()
    );
    Ok((html, doc))
}

/// Print Typst diagnostics to stderr with source context (rustc-style).
fn emit(world: &dyn DiagnosticWorld, diags: &EcoVec<SourceDiagnostic>) {
    let mut out = StandardStream::stderr(ColorChoice::Auto);
    let _ = diagnostics::emit(&mut out, world, diags.iter(), DiagnosticFormat::Human);
}

/// Show real warnings (always), but filter the constant "HTML export is
/// experimental" notice so normal builds stay quiet.
fn report_warnings(world: &dyn DiagnosticWorld, warnings: &EcoVec<SourceDiagnostic>) {
    let real: Vec<&SourceDiagnostic> = warnings
        .iter()
        .filter(|w| !is_html_experimental_notice(w.message.as_str()))
        .collect();
    if real.is_empty() {
        return;
    }
    let mut out = StandardStream::stderr(ColorChoice::Auto);
    let _ = diagnostics::emit(&mut out, world, real, DiagnosticFormat::Human);
}

fn is_html_experimental_notice(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("html export") && (m.contains("experiment") || m.contains("development"))
}

/// Extract the value of the `<frontmatter>` metadata element, as JSON.
fn query_frontmatter(doc: &HtmlDocument) -> Option<serde_json::Value> {
    let label = Label::new(PicoStr::intern("frontmatter"))?;
    let hits = doc.introspector().query(&Selector::Label(label));
    let value: Value = hits.first()?.field_by_name("value").ok()?;
    serde_json::to_value(value).ok()
}

fn inject_css(html: &str) -> String {
    html.replacen(
        "</head>",
        "    <link rel=\"stylesheet\" href=\"/style.css\">\n  </head>",
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_css_goes_into_head() {
        let out = inject_css("<head>\n  </head><body>x</body>");
        assert!(out.contains(r#"<link rel="stylesheet" href="/style.css">"#));
        assert!(out.find("stylesheet").unwrap() < out.find("</head>").unwrap());
    }

    #[test]
    fn experimental_notice_is_recognized() {
        assert!(is_html_experimental_notice(
            "HTML export is under active development"
        ));
        assert!(!is_html_experimental_notice("equation was ignored"));
    }
}
