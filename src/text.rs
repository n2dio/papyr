//! Small text utilities shared across modules.

/// URL/filename-safe slug. MUST match the Typst `slugify` in lib/template.typ
/// (the build-time link checker verifies they agree): lowercase, each run of
/// non-`[a-z0-9]` becomes a single `-`, leading/trailing `-` trimmed.
pub(crate) fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Decode the handful of HTML entities Typst emits in text, so text extracted
/// from generated HTML (heading labels, `<title>`) is the real string before we
/// re-escape it for another context. `&amp;` is decoded last so already-decoded
/// text isn't decoded twice.
pub(crate) fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_entities_reverses_typst_escaping() {
        assert_eq!(
            decode_entities("Rust &amp; You &lt;ok&gt;"),
            "Rust & You <ok>"
        );
        assert_eq!(decode_entities("plain"), "plain");
        // a literal "&lt;" in the source becomes "&amp;lt;" → decodes to "&lt;"
        assert_eq!(decode_entities("&amp;lt;"), "&lt;");
    }

    #[test]
    fn slugify_matches_typst_rules() {
        assert_eq!(slugify("Machine Learning"), "machine-learning");
        assert_eq!(slugify("C++"), "c");
        assert_eq!(slugify("a/b"), "a-b");
        assert_eq!(slugify("  Hello,  World!  "), "hello-world");
        assert_eq!(slugify("café"), "caf");
        assert_eq!(slugify("Rust 2024"), "rust-2024");
        assert_eq!(slugify("--x--"), "x");
        assert_eq!(slugify("!!!"), "");
    }
}
