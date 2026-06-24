//! Build-time table of contents and heading anchors for posts.
//!
//! Typst emits plain `<h2>`/`<h3>` (no ids), so we add an `id` and a hover
//! anchor to each, and — when a post opts in and has at least two — inject a
//! `<details>` table of contents at the top of the article (expanded by
//! default; `collapsed: true` starts it folded). All zero-JS.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::{feed, text};

/// A whole `<h2>`/`<h3>…</h2|3>` element (Typst emits these without attributes).
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<h([23])>(.*?)</h[23]>").unwrap());
/// Any HTML tag, for reducing a heading's inner markup to plain text.
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

/// Whether a post shows a table of contents, and its initial state.
#[derive(Clone, Copy)]
pub(crate) enum Toc {
    Off,
    Open,
    Collapsed,
}

impl Toc {
    /// `Some(open)` when a TOC should be shown; `open` is its initial state.
    fn enabled(self) -> Option<bool> {
        match self {
            Toc::Off => None,
            Toc::Open => Some(true),
            Toc::Collapsed => Some(false),
        }
    }
}

/// Add ids + hover anchors to a post's h2/h3 headings. When `toc` enables it and
/// there are two or more headings, also inject a TOC at the top of the article.
/// Only h2/h3 are touched.
pub(crate) fn process(html: &str, toc: Toc) -> String {
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
    if let Some(open) = toc.enabled() {
        if headings.len() >= 2 {
            let toc_html = build_toc(&headings, open);
            if let Some(pos) = toc_insert_pos(&result) {
                result.insert_str(pos, &toc_html);
            }
        }
    }
    result
}

/// Where the TOC goes: at the top of the article, just after the post-meta line
/// — not buried below the intro paragraph. Falls back to before the first
/// heading if there's no post meta.
fn toc_insert_pos(html: &str) -> Option<usize> {
    if let Some(i) = html.find(r#"class="post-meta""#) {
        if let Some(end) = html[i..].find("</p>") {
            return Some(i + end + "</p>".len());
        }
    }
    html.find(r#"<h2 id=""#)
        .or_else(|| html.find(r#"<h3 id=""#))
}

fn build_toc(headings: &[(u8, String, String)], open: bool) -> String {
    let mut s = String::from(if open {
        r#"<details class="toc" open><summary>Contents</summary><ul>"#
    } else {
        r#"<details class="toc"><summary>Contents</summary><ul>"#
    });
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
        let out = process(html, Toc::Open);
        assert!(out.contains(
            r##"<h2 id="first-section">First Section <a class="anchor" href="#first-section">#</a></h2>"##
        ));
        assert!(out.contains(r#"<details class="toc" open>"#));
        assert!(out.contains(r##"<a href="#second">Second</a>"##));
        assert!(out.contains(r##"<a href="#sub">Sub</a>"##));
        // TOC sits before the first heading
        assert!(out.find("toc").unwrap() < out.find(r#"id="first-section""#).unwrap());
    }

    #[test]
    fn toc_is_opt_in_but_anchors_are_not() {
        // Same content, toc disabled: headings still get ids + anchors, no TOC.
        let html = "<main><h2>First Section</h2><h2>Second</h2></main>";
        let out = process(html, Toc::Off);
        assert!(out.contains(r#"id="first-section""#));
        assert!(out.contains(r##"<a class="anchor" href="#first-section">#</a>"##));
        assert!(!out.contains("toc"));
    }

    #[test]
    fn no_toc_for_a_single_heading_but_still_anchored() {
        let out = process("<main><h2>Only</h2></main>", Toc::Open);
        assert!(out.contains(r#"id="only""#));
        assert!(!out.contains("toc"));
    }

    #[test]
    fn toc_sits_after_meta_above_the_intro() {
        // With a post-meta line, the TOC goes at the top of the article — after
        // the meta, before the intro paragraph — not below the first paragraph.
        let html = r#"<article><h1 class="post-title">T</h1><p class="post-meta">d</p><p>intro</p><h2>One</h2><h2>Two</h2></article>"#;
        let out = process(html, Toc::Open);
        let toc = out.find("toc").unwrap();
        assert!(toc > out.find("post-meta").unwrap());
        assert!(toc < out.find("intro").unwrap());
    }

    #[test]
    fn collapsed_starts_without_the_open_attribute() {
        let out = process("<main><h2>A</h2><h2>B</h2></main>", Toc::Collapsed);
        assert!(out.contains(r#"<details class="toc"><summary>"#));
        assert!(!out.contains(r#"class="toc" open"#));
    }

    #[test]
    fn dedupes_repeated_ids() {
        let out = process("<main><h2>Dup</h2><h2>Dup</h2></main>", Toc::Open);
        assert!(out.contains(r#"id="dup""#));
        assert!(out.contains(r#"id="dup-1""#));
    }

    #[test]
    fn strips_inline_tags_for_text_and_id() {
        assert_eq!(strip_tags("Some <code>code</code> here"), "Some code here");
        let out = process(
            "<main><h2>A <code>b</code> C</h2><h2>two</h2></main>",
            Toc::Open,
        );
        assert!(out.contains(r#"id="a-b-c""#));
    }

    #[test]
    fn leading_h3_does_not_orphan_a_nested_list() {
        // A post that opens on h3 (before any h2) must not produce <ul><ul>.
        let out = process("<main><h3>Early</h3><h2>Main</h2></main>", Toc::Open);
        assert!(out.contains(r#"class="toc" open>"#));
        assert!(!out.contains("<ul><ul>"));
        assert!(out.contains(r##"<a href="#early">Early</a>"##));
    }

    #[test]
    fn decodes_entities_for_id_and_toc_label() {
        let out = process("<main><h2>Tom &amp; Jerry</h2><h2>x</h2></main>", Toc::Open);
        assert!(out.contains(r#"id="tom-jerry""#)); // not "tom-amp-jerry"
        assert!(out.contains(r##"<a href="#tom-jerry">Tom &amp; Jerry</a>"##));
    }
}
