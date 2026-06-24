//! The `init` command: scaffold a fresh papyr site in a directory.
//!
//! The generic engine files (template, listing pages, CSS, code theme, fonts)
//! are embedded from this repo so they stay in sync with the tool. The starter
//! content (config, example post, pages) is generic placeholder text.

use std::fs;
use std::path::{Path, PathBuf};

use crate::Res;

// --- embedded engine files (site-agnostic) ---------------------------------
const TEMPLATE: &str = include_str!("../lib/template.typ");
const GEN_INDEX: &str = include_str!("../gen/index.typ");
const GEN_TAG: &str = include_str!("../gen/tag.typ");
const GEN_TAGS_INDEX: &str = include_str!("../gen/tags-index.typ");
const STYLE: &str = include_str!("../assets/style.css");
const CODE_THEME: &str = include_str!("../assets/code-theme.tmTheme");

const FONTS: &[(&str, &[u8])] = &[
    (
        "assets/fonts/ibm-plex-sans-400.woff2",
        include_bytes!("../assets/fonts/ibm-plex-sans-400.woff2"),
    ),
    (
        "assets/fonts/ibm-plex-sans-500.woff2",
        include_bytes!("../assets/fonts/ibm-plex-sans-500.woff2"),
    ),
    (
        "assets/fonts/ibm-plex-sans-600.woff2",
        include_bytes!("../assets/fonts/ibm-plex-sans-600.woff2"),
    ),
    (
        "assets/fonts/ibm-plex-sans-700.woff2",
        include_bytes!("../assets/fonts/ibm-plex-sans-700.woff2"),
    ),
    (
        "assets/fonts/jetbrains-mono-400.woff2",
        include_bytes!("../assets/fonts/jetbrains-mono-400.woff2"),
    ),
    (
        "assets/fonts/jetbrains-mono-700.woff2",
        include_bytes!("../assets/fonts/jetbrains-mono-700.woff2"),
    ),
];

// --- starter content (generic placeholders), embedded from scaffold/ --------
const CONFIG: &str = include_str!("../scaffold/config.yaml");
const PAGE_ABOUT: &str = include_str!("../scaffold/about.typ");
const PAGE_IMPRINT: &str = include_str!("../scaffold/imprint.typ");
const STARTER_POST: &str = include_str!("../scaffold/hello-papyr.typ");
const GITIGNORE: &str = include_str!("../scaffold/gitignore");

/// The example post with today's date filled into its `{date}` placeholder.
fn starter_post(today: &str) -> String {
    STARTER_POST.replace("{date}", today)
}

pub fn init(dir: &Path) -> Res<()> {
    if dir.join("config.yaml").exists() {
        return Err(format!(
            "{} already contains a papyr site (config.yaml exists)",
            dir.display()
        )
        .into());
    }

    let post = starter_post(&crate::dates::today());
    let text_files: [(&str, &str); 10] = [
        ("config.yaml", CONFIG),
        ("lib/template.typ", TEMPLATE),
        ("gen/index.typ", GEN_INDEX),
        ("gen/tag.typ", GEN_TAG),
        ("gen/tags-index.typ", GEN_TAGS_INDEX),
        ("assets/style.css", STYLE),
        ("assets/code-theme.tmTheme", CODE_THEME),
        ("posts/hello-papyr.typ", &post),
        ("pages/about.typ", PAGE_ABOUT),
        ("pages/imprint.typ", PAGE_IMPRINT),
    ];

    for (rel, content) in text_files {
        write(dir.join(rel), content.as_bytes())?;
    }
    write(dir.join(".gitignore"), GITIGNORE.as_bytes())?;
    for (rel, bytes) in FONTS {
        write(dir.join(rel), bytes)?;
    }

    let where_ = dir.display();
    println!("✓ scaffolded a new papyr site in {where_}");
    println!("  next:");
    if dir != Path::new(".") {
        println!("    cd {where_}");
    }
    println!("    papyr serve     # then open http://localhost:8080");
    Ok(())
}

fn write(path: PathBuf, bytes: &[u8]) -> Res<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}
