//! Build-time table of contents and heading anchors for posts.
//!
//! Typst emits plain `<h2>`/`<h3>` (no ids), so we add an `id` and a hover
//! anchor to each, and — when a post has at least two — inject a collapsible
//! `<details>` table of contents before the first heading. All zero-JS.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::{feed, text};

/// A whole `<h2>`/`<h3>…</h2|3>` element (Typst emits these without attributes).
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<h([23])>(.*?)</h[23]>").unwrap());
/// Any HTML tag, for reducing a heading's inner markup to plain text.
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

/// Add ids + hover anchors to a post's h2/h3 headings. When `with_toc` is set
/// (the post's `toc: true` frontmatter) and there are two or more headings,
/// also inject a collapsible TOC before the first one. Only h2/h3 are touched.
pub(crate) fn process(html: &str, with_toc: bool) -> String {
    let mut headings: Vec<(u8, String, String)> = Vec::new();
    let mut seen: HashMap<String, u32> = HashMap::new();

    let result = HEADING.replace_all(html, |caps: &Captures| {
        let level: u8 = if &caps[1] == "2" { 2 } else { 3 };
        let inner = &caps[2];
        let text_plain = text::decode_entities(&strip_tags(inner));

        let base = text::slugify(&text_plain);
        let base = if base.is_empty() {
            format!("section-{}", headings.len() + 1)
        } else {
            base
        };
        let n = seen.entry(base.clone()).or_insert(0);
        let id = if *n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        *n += 1;

        let rewritten = format!(
            r##"<h{level} id="{id}">{inner} <a class="anchor" href="#{id}">#</a></h{level}>"##
        );
        headings.push((level, text_plain, id));
        rewritten
    });

    let mut result = result.into_owned();
    if with_toc && headings.len() >= 2 {
        let toc = build_toc(&headings);
        if let Some(pos) = result
            .find(r#"<h2 id=""#)
            .or_else(|| result.find(r#"<h3 id=""#))
        {
            result.insert_str(pos, &toc);
        }
    }
    result
}

fn build_toc(headings: &[(u8, String, String)]) -> String {
    let mut s = String::from(r#"<details class="toc"><summary>Contents</summary><ul>"#);
    let mut li_open = false;
    let mut sub_open = false;
    for (level, text_plain, id) in headings {
        let link = format!(r##"<a href="#{id}">{}</a>"##, feed::xml(text_plain));
        // Treat an h3 with no open parent (a post that opens on h3) as top-level,
        // so we never nest a <ul> directly inside a <ul> without an <li>.
        if *level <= 2 || !li_open {
            if sub_open {
                s.push_str("</ul>");
                sub_open = false;
            }
            if li_open {
                s.push_str("</li>");
            }
            s.push_str(&format!("<li>{link}"));
            li_open = true;
        } else {
            if !sub_open {
                s.push_str("<ul>");
                sub_open = true;
            }
            s.push_str(&format!("<li>{link}</li>"));
        }
    }
    if sub_open {
        s.push_str("</ul>");
    }
    if li_open {
        s.push_str("</li>");
    }
    s.push_str("</ul></details>");
    s
}

/// Strip inline tags from a heading's inner HTML to get its plain text.
fn strip_tags(s: &str) -> String {
    TAG.replace_all(s, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_ids_anchors_and_toc() {
        let html = "<main><h2>First Section</h2><p>x</p><h2>Second</h2><h3>Sub</h3></main>";
        let out = process(html, true);
        assert!(out.contains(
            r##"<h2 id="first-section">First Section <a class="anchor" href="#first-section">#</a></h2>"##
        ));
        assert!(out.contains(r#"<details class="toc">"#));
        assert!(out.contains(r##"<a href="#second">Second</a>"##));
        assert!(out.contains(r##"<a href="#sub">Sub</a>"##));
        // TOC sits before the first heading
        assert!(out.find("toc").unwrap() < out.find(r#"id="first-section""#).unwrap());
    }

    #[test]
    fn toc_is_opt_in_but_anchors_are_not() {
        // Same content, toc disabled: headings still get ids + anchors, no TOC.
        let html = "<main><h2>First Section</h2><h2>Second</h2></main>";
        let out = process(html, false);
        assert!(out.contains(r#"id="first-section""#));
        assert!(out.contains(r##"<a class="anchor" href="#first-section">#</a>"##));
        assert!(!out.contains("toc"));
    }

    #[test]
    fn no_toc_for_a_single_heading_but_still_anchored() {
        let out = process("<main><h2>Only</h2></main>", true);
        assert!(out.contains(r#"id="only""#));
        assert!(!out.contains("toc"));
    }

    #[test]
    fn dedupes_repeated_ids() {
        let out = process("<main><h2>Dup</h2><h2>Dup</h2></main>", true);
        assert!(out.contains(r#"id="dup""#));
        assert!(out.contains(r#"id="dup-1""#));
    }

    #[test]
    fn strips_inline_tags_for_text_and_id() {
        assert_eq!(strip_tags("Some <code>code</code> here"), "Some code here");
        let out = process("<main><h2>A <code>b</code> C</h2><h2>two</h2></main>", true);
        assert!(out.contains(r#"id="a-b-c""#));
    }

    #[test]
    fn leading_h3_does_not_orphan_a_nested_list() {
        // A post that opens on h3 (before any h2) must not produce <ul><ul>.
        let out = process("<main><h3>Early</h3><h2>Main</h2></main>", true);
        assert!(out.contains(r#"<details class="toc">"#));
        assert!(!out.contains("<ul><ul>"));
        assert!(out.contains(r##"<a href="#early">Early</a>"##));
    }

    #[test]
    fn decodes_entities_for_id_and_toc_label() {
        let out = process("<main><h2>Tom &amp; Jerry</h2><h2>x</h2></main>", true);
        assert!(out.contains(r#"id="tom-jerry""#)); // not "tom-amp-jerry"
        assert!(out.contains(r##"<a href="#tom-jerry">Tom &amp; Jerry</a>"##));
    }
}
