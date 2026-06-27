//! Data structures shared across the build.

use serde::{Deserialize, Serialize};

/// Site-wide configuration, read from `config.yaml`. Only the fields the
/// feed needs are declared here; the Typst templates read the rest (e.g.
/// `tagline`, `author`) directly via `yaml("/config.yaml")`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    /// Pre-rendered `<link>` tags for whichever favicon files exist in
    /// `assets/`. Populated by the build, not read from `config.yaml`.
    #[serde(skip)]
    pub favicons: Vec<String>,
}

/// A post's frontmatter, as written in the `metadata((...))` block.
#[derive(Debug, Clone, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    pub date: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: String,
    /// Opt in to a table of contents (`toc: true` in the post).
    #[serde(default)]
    pub toc: bool,
    /// When a TOC is shown, start it collapsed instead of expanded.
    #[serde(default)]
    pub collapsed: bool,
    /// Number the lines of this post's code blocks.
    #[serde(default, rename = "line-numbers")]
    pub line_numbers: bool,
}

/// A post's metadata plus its slug; this is what we serialize to
/// `build/posts.json` for the Typst listing templates to read.
#[derive(Debug, Clone, Serialize)]
pub struct PostMeta {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub summary: String,
}

impl PostMeta {
    pub fn from_frontmatter(slug: String, fm: FrontMatter) -> Self {
        PostMeta {
            slug,
            title: fm.title,
            date: fm.date,
            tags: fm.tags,
            summary: fm.summary,
        }
    }
}
