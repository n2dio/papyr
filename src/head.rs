//! Per-page `<head>` metadata: description, canonical URL, OpenGraph/Twitter
//! cards, theme-color, and RSS feed autodiscovery. Injected before `</head>`
//! (the stylesheet `<link>` is already added in `render`).

use std::fmt::Write;
use std::sync::LazyLock;

use regex::Regex;

use crate::model::Config;
use crate::text;

/// Inject SEO/social `<head>` tags into a page's HTML. `canonical` is the page's
/// absolute URL; `description` is the post summary (or site description);
/// `is_article` distinguishes posts from listings/pages.
pub(crate) fn meta(
    html: &str,
    config: &Config,
    canonical: &str,
    description: &str,
    is_article: bool,
) -> String {
    // The <title> is already HTML-escaped by Typst; decode it before attr() so
    // it isn't double-escaped in og:title.
    let title = extract_title(html)
        .map(|t| text::decode_entities(&t))
        .unwrap_or_else(|| config.title.clone());
    let og_type = if is_article { "article" } else { "website" };
    let (t, d, c, site) = (
        attr(&title),
        attr(description),
        attr(canonical),
        attr(&config.title),
    );

    let mut s = String::new();
    let mut line = |l: &str| {
        let _ = writeln!(s, "    {l}");
    };
    line(&format!(r#"<meta name="description" content="{d}">"#));
    line(&format!(r#"<link rel="canonical" href="{c}">"#));
    line(r##"<meta name="theme-color" media="(prefers-color-scheme: light)" content="#fcfcfb">"##);
    line(r##"<meta name="theme-color" media="(prefers-color-scheme: dark)" content="#0f0f12">"##);
    line(&format!(r#"<meta property="og:type" content="{og_type}">"#));
    line(&format!(r#"<meta property="og:title" content="{t}">"#));
    line(&format!(
        r#"<meta property="og:description" content="{d}">"#
    ));
    line(&format!(r#"<meta property="og:url" content="{c}">"#));
    line(&format!(
        r#"<meta property="og:site_name" content="{site}">"#
    ));
    line(r#"<meta name="twitter:card" content="summary">"#);
    line(&format!(
        r#"<link rel="alternate" type="application/rss+xml" title="{site}" href="/feed.xml">"#
    ));
    s.push_str("  </head>");

    html.replacen("</head>", &s, 1)
}

fn extract_title(html: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<title>(.*?)</title>").unwrap());
    RE.captures(html).map(|c| c[1].to_string())
}

/// Escape a string for use in an HTML attribute value.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            title: "My \"Blog\"".into(),
            url: "https://example.com".into(),
            description: "desc".into(),
        }
    }

    #[test]
    fn injects_before_head_close_with_escaping() {
        let html = "<head>\n    <title>Hello — My Blog</title>\n  </head><body></body>";
        let out = meta(
            html,
            &cfg(),
            "https://example.com/posts/x.html",
            "A & B",
            true,
        );
        assert!(out.contains(r#"<meta property="og:type" content="article">"#));
        assert!(out.contains(r#"content="A &amp; B""#));
        assert!(out.contains(r#"og:title" content="Hello — My Blog""#));
        assert!(out.contains(r#"og:site_name" content="My &quot;Blog&quot;""#));
        assert!(out.contains(r#"href="https://example.com/posts/x.html""#));
        assert!(out.contains(r#"rel="alternate" type="application/rss+xml""#));
        // exactly one </head>, and our block precedes it
        assert_eq!(out.matches("</head>").count(), 1);
        assert!(out.find("og:url").unwrap() < out.find("</head>").unwrap());
    }

    #[test]
    fn og_title_is_not_double_escaped() {
        // Typst emits an escaped <title>; og:title must escape exactly once.
        let html = "<head><title>Rust &amp; You &lt;ok&gt;</title></head>";
        let out = meta(html, &cfg(), "https://example.com/", "d", true);
        assert!(out.contains(r#"og:title" content="Rust &amp; You &lt;ok&gt;""#));
        assert!(!out.contains("&amp;amp;"));
    }

    #[test]
    fn website_type_for_non_articles() {
        let html = "<head><title>T</title></head>";
        let out = meta(html, &cfg(), "https://example.com/", "d", false);
        assert!(out.contains(r#"og:type" content="website""#));
    }
}
