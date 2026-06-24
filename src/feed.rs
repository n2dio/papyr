//! RSS 2.0 feed generation.

use std::fmt::Write;

use crate::dates;
use crate::model::{Config, PostMeta};

/// Render an RSS 2.0 feed for the posts (newest first).
pub(crate) fn render_feed(config: &Config, posts: &[PostMeta]) -> String {
    let mut s = String::new();
    // Writing to a String is infallible, so the fmt errors can't occur.
    write_feed(&mut s, config, posts).expect("formatting to a String never fails");
    s
}

fn write_feed(s: &mut String, config: &Config, posts: &[PostMeta]) -> std::fmt::Result {
    let url = config.url.trim_end_matches('/');
    writeln!(s, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(s, "<rss version=\"2.0\">\n  <channel>")?;
    writeln!(s, "    <title>{}</title>", xml(&config.title))?;
    writeln!(s, "    <link>{}</link>", xml(url))?;
    writeln!(
        s,
        "    <description>{}</description>",
        xml(&config.description)
    )?;
    for p in posts {
        let link = format!("{url}/posts/{}.html", p.slug);
        writeln!(s, "    <item>")?;
        writeln!(s, "      <title>{}</title>", xml(&p.title))?;
        writeln!(s, "      <link>{}</link>", xml(&link))?;
        writeln!(s, "      <guid>{}</guid>", xml(&link))?;
        writeln!(s, "      <pubDate>{}</pubDate>", dates::rfc2822(&p.date))?;
        writeln!(s, "      <description>{}</description>", xml(&p.summary))?;
        writeln!(s, "    </item>")?;
    }
    writeln!(s, "  </channel>\n</rss>")?;
    Ok(())
}

pub(crate) fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escapes_specials() {
        assert_eq!(xml("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        assert_eq!(xml("plain"), "plain");
    }
}
