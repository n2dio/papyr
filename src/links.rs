//! Build-time internal link checking.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::Res;

/// Warn about internal links (`href="/..."`) that don't resolve to a file in
/// the output. Catches typos and renamed/removed posts and pages. Returns the
/// number of broken links found.
pub(crate) fn check_links(out: &Path) -> Res<usize> {
    let mut html_files = Vec::new();
    collect_html(out, &mut html_files)?;

    let mut broken = 0usize;
    for file in &html_files {
        let content = fs::read_to_string(file)?;
        let page = file.strip_prefix(out).unwrap_or(file).display();
        for href in internal_hrefs(&content) {
            if !target_exists(out, &href) {
                eprintln!("⚠ broken link in {page}: {href}");
                tracing::warn!(%page, %href, "broken internal link");
                broken += 1;
            }
        }
    }
    if broken == 0 {
        tracing::debug!("internal links OK ({} pages)", html_files.len());
    } else {
        eprintln!("⚠ {broken} broken internal link(s)");
    }
    Ok(broken)
}

pub(crate) fn collect_html(dir: &Path, out: &mut Vec<PathBuf>) -> Res<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_html(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("html") {
            out.push(path);
        }
    }
    Ok(())
}

/// Extract `href="/..."` values (site-internal absolute links), minus
/// fragments and queries. Ignores external and protocol-relative URLs.
fn internal_hrefs(html: &str) -> Vec<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"href="(/[^"]*)""#).unwrap());
    RE.captures_iter(html)
        .filter_map(|c| {
            let raw = &c[1];
            if raw.starts_with("//") {
                return None; // protocol-relative, treat as external
            }
            Some(raw.split(['#', '?']).next().unwrap_or(raw).to_string())
        })
        .collect()
}

fn target_exists(out: &Path, href: &str) -> bool {
    let rel = href.trim_start_matches('/');
    let path = out.join(rel);
    if href.ends_with('/') || rel.is_empty() {
        path.join("index.html").is_file()
    } else {
        path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_hrefs_keeps_only_site_absolute() {
        let html = r#"<a href="/posts/x.html">x</a><a href="https://e.com">e</a><a href="/t/#f">f</a><a href="//cdn/a">c</a>"#;
        assert_eq!(internal_hrefs(html), vec!["/posts/x.html", "/t/"]);
    }
}
