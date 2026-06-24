//! Optional line numbers for code blocks (zero-JS, via CSS counters).
//!
//! Typst emits `<pre><code …>line1\nline2…</code></pre>` with no per-line
//! element, so to number lines we wrap each line in `<span class="line">` and
//! tag the `<pre>`. The stylesheet then renders a gutter with CSS counters.

use std::fmt::Write;
use std::sync::LazyLock;

use regex::{Captures, Regex};

/// A whole code block. Typst emits `<pre>` and `<code …>` adjacent, with the
/// language (if any) as a `data-lang` attribute on the `<code>`.
static CODE_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<pre>(<code[^>]*>)(.*?)</code></pre>").unwrap());

/// Wrap each line of every code block in `<span class="line">` and mark the
/// `<pre>` so the stylesheet can number lines. Inline `<code>` is untouched
/// (it has no `<pre>`).
pub(crate) fn number_lines(html: &str) -> String {
    CODE_BLOCK
        .replace_all(html, |c: &Captures| {
            let code_open = &c[1];
            // Drop one trailing newline so a block ending in "\n" doesn't get a
            // spurious empty final line.
            let body = &c[2];
            let body = body.strip_suffix('\n').unwrap_or(body);

            let mut lines = String::new();
            for (i, line) in body.split('\n').enumerate() {
                if i > 0 {
                    lines.push('\n');
                }
                let _ = write!(lines, r#"<span class="line">{line}</span>"#);
            }
            format!(r#"<pre class="numbered">{code_open}{lines}</code></pre>"#)
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_each_line_and_marks_the_pre() {
        let html = "<pre><code data-lang=\"rust\">fn a() {\n    b();\n}</code></pre>";
        let out = number_lines(html);
        assert!(out.starts_with(r#"<pre class="numbered"><code data-lang="rust">"#));
        assert_eq!(out.matches(r#"<span class="line">"#).count(), 3);
        assert!(out.contains(r#"<span class="line">}</span>"#));
        // newlines between lines are preserved (so `white-space: pre` still breaks)
        assert!(out.contains("</span>\n<span"));
    }

    #[test]
    fn a_trailing_newline_does_not_add_a_blank_line() {
        let out = number_lines("<pre><code>only</code></pre>");
        assert_eq!(out.matches(r#"<span class="line">"#).count(), 1);
        let out = number_lines("<pre><code>only\n</code></pre>");
        assert_eq!(out.matches(r#"<span class="line">"#).count(), 1);
    }

    #[test]
    fn inline_code_is_left_alone() {
        let html = "<p>use <code>x()</code> here</p>";
        assert_eq!(number_lines(html), html);
    }
}
