//! The `build` command: orchestrate compiling all sources into `site/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use typst::foundations::{Dict, Value};

use crate::model::{Config, PostMeta};
use crate::world::Shared;
use crate::{code, dates, feed, head, links, render, text, toc, Res};

/// A compiled post held until we know its neighbors: its HTML, metadata, and
/// the post-level rendering options still needed at write time.
struct Compiled {
    html: String,
    meta: PostMeta,
    toc: toc::Toc,
    line_numbers: bool,
}

/// Build the whole site rooted at `root` into `root/site`. With `strict`, a
/// broken internal link fails the build (before publishing the new output).
pub fn build(root: &Path, strict: bool) -> Res<()> {
    let shared = Shared::new(root.to_path_buf());
    let site = root.join("site");
    let build_dir = root.join("build");
    // Build into a staging dir (under the gitignored build/), then swap it into
    // `site/` at the end. Keeps `site/` whole while a rebuild is in flight.
    let staging = build_dir.join("_site");

    println!("› cleaning");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(staging.join("posts"))?;
    fs::create_dir_all(staging.join("tags"))?;

    let mut config = read_config(root)?;
    config.favicons = favicon_links(root);

    println!("› compiling posts");
    let posts = compile_posts(&shared, root, &staging, &config)?;
    let n = posts.len();
    println!("  {n} post{}", if n == 1 { "" } else { "s" });

    println!("› generating index, tags, pages");
    generate_listings(&shared, &staging, &build_dir, &posts, &config)?;
    compile_pages(&shared, root, &staging, &config)?;

    copy_assets(root, &staging)?;

    println!("› generating feed.xml, sitemap.xml, robots.txt");
    fs::write(staging.join("feed.xml"), feed::render_feed(&config, &posts))?;
    write_sitemap(&staging, &config)?;
    write_robots(&staging, &config)?;

    let broken = links::check_links(&staging)?;
    if strict && broken > 0 {
        return Err(format!("{broken} broken internal link(s); failing due to --strict").into());
    }

    // Swap the freshly built site into place. The window where `site/` is absent
    // is two rename() calls, not the whole rebuild — and a failed build above
    // returns early, leaving the previous `site/` untouched.
    swap_into_place(&staging, &site, &build_dir)?;

    // Bound the memoization cache so long-running watch sessions don't grow
    // unbounded; results stay valid because inputs are content-hashed.
    comemo::evict(10);

    println!("✓ built site → {}", site.display());
    Ok(())
}

/// Compile every `posts/*.typ` into `staging/posts/`, returning their metadata
/// sorted newest-first.
fn compile_posts(
    shared: &Shared,
    root: &Path,
    staging: &Path,
    config: &Config,
) -> Res<Vec<PostMeta>> {
    // Compile every post first — we need all dates before we know each post's
    // neighbors for prev/next.
    let mut compiled: Vec<Compiled> = Vec::new();
    for entry in fs::read_dir(root.join("posts"))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("typ") {
            continue;
        }
        let Some(slug) = path.file_stem().and_then(|s| s.to_str()) else {
            eprintln!("⚠ skipping {}: non-UTF-8 filename", path.display());
            continue;
        };
        let slug = slug.to_owned();
        let rel = format!("posts/{slug}.typ");
        warn_unsafe_filename(&rel, &slug);

        let (html, fm) = render::render_post(shared, &rel)?;
        if dates::parse(&fm.date).is_none() {
            eprintln!("⚠ {rel}: \"{}\" is not a valid date or timestamp", fm.date);
        }
        let toc = match (fm.toc, fm.collapsed) {
            (false, _) => toc::Toc::Off,
            (true, false) => toc::Toc::Open,
            (true, true) => toc::Toc::Collapsed,
        };
        let line_numbers = fm.line_numbers;
        compiled.push(Compiled {
            html,
            meta: PostMeta::from_frontmatter(slug, fm),
            toc,
            line_numbers,
        });
    }

    // Newest first; unparseable dates sort last. Cached so each date parses once.
    compiled.sort_by_cached_key(|c| std::cmp::Reverse(dates::parse(&c.meta.date)));

    // Write each post with prev (older) / next (newer) navigation.
    for i in 0..compiled.len() {
        let older = compiled.get(i + 1).map(|c| &c.meta);
        let newer = i
            .checked_sub(1)
            .and_then(|j| compiled.get(j))
            .map(|c| &c.meta);
        let c = &compiled[i];
        let html = toc::process(&c.html, c.toc);
        let html = if c.line_numbers {
            code::number_lines(&html)
        } else {
            html
        };
        let html = inject_post_nav(&html, older, newer);
        write_html(
            staging,
            &format!("posts/{}.html", c.meta.slug),
            &html,
            config,
            &c.meta.summary,
            true,
        )?;
    }

    Ok(compiled.into_iter().map(|c| c.meta).collect())
}

/// Insert a prev (older) / next (newer) navigation block before `</main>`.
fn inject_post_nav(html: &str, older: Option<&PostMeta>, newer: Option<&PostMeta>) -> String {
    if older.is_none() && newer.is_none() {
        return html.to_string();
    }
    let mut nav = String::from(r#"<nav class="post-nav">"#);
    if let Some(o) = older {
        nav.push_str(&format!(
            r#"<a class="prev" href="/posts/{}.html"><span>← Older</span>{}</a>"#,
            o.slug,
            feed::xml(&o.title)
        ));
    }
    if let Some(n) = newer {
        nav.push_str(&format!(
            r#"<a class="next" href="/posts/{}.html"><span>Newer →</span>{}</a>"#,
            n.slug,
            feed::xml(&n.title)
        ));
    }
    nav.push_str("</nav>");
    html.replacen("</main>", &format!("{nav}</main>"), 1)
}

/// Write `posts.json` and generate the home page, tag pages, and tag index.
fn generate_listings(
    shared: &Shared,
    staging: &Path,
    build_dir: &Path,
    posts: &[PostMeta],
    config: &Config,
) -> Res<()> {
    // Hand the listing templates their data.
    fs::write(
        build_dir.join("posts.json"),
        serde_json::to_vec_pretty(posts)?,
    )?;

    let index = render::render_page(shared, "gen/index.typ", None)?;
    write_html(
        staging,
        "index.html",
        &index,
        config,
        &config.description,
        false,
    )?;

    let tags_index = render::render_page(shared, "gen/tags-index.typ", None)?;
    write_html(
        staging,
        "tags/index.html",
        &tags_index,
        config,
        &config.description,
        false,
    )?;

    // Group tags by slug (deterministic, sorted); keep the first raw tag as the
    // display name. Slugs are collision-safe: distinct tags that slugify to the
    // same value share one page.
    let mut tag_pages: BTreeMap<String, String> = BTreeMap::new();
    for p in posts {
        for t in &p.tags {
            let slug = text::slugify(t);
            if !slug.is_empty() {
                tag_pages.entry(slug).or_insert_with(|| t.clone());
            }
        }
    }
    for (slug, name) in &tag_pages {
        let mut inputs = Dict::new();
        inputs.insert("slug".into(), Value::Str(slug.as_str().into()));
        inputs.insert("name".into(), Value::Str(name.as_str().into()));
        let html = render::render_page(shared, "gen/tag.typ", Some(inputs))?;
        write_html(
            staging,
            &format!("tags/{slug}.html"),
            &html,
            config,
            &config.description,
            false,
        )?;
    }
    Ok(())
}

/// Compile every `pages/*.typ` into the site root (e.g. `about.html`).
fn compile_pages(shared: &Shared, root: &Path, staging: &Path, config: &Config) -> Res<()> {
    let pages = root.join("pages");
    if !pages.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(pages)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("typ") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            eprintln!("⚠ skipping {}: non-UTF-8 filename", path.display());
            continue;
        };
        let rel = format!("pages/{stem}.typ");
        warn_unsafe_filename(&rel, stem);
        let html = render::render_page(shared, &rel, None)?;
        write_html(
            staging,
            &format!("{stem}.html"),
            &html,
            config,
            &config.description,
            false,
        )?;
    }
    Ok(())
}

/// Nudge the author if a source filename isn't already URL-safe: the file stem
/// becomes the slug verbatim (unlike tags, which are slugified), so a name with
/// spaces or uppercase yields a fragile URL like `/posts/My Post.html`.
fn warn_unsafe_filename(rel: &str, stem: &str) {
    let safe = text::slugify(stem);
    if stem != safe {
        eprintln!("⚠ {rel}: filename isn't URL-safe; consider renaming to \"{safe}\"");
    }
}

/// Page's absolute canonical URL from its site-relative output path.
fn canonical_url(config: &Config, rel: &str) -> String {
    let url = config.url.trim_end_matches('/');
    if rel == "index.html" {
        format!("{url}/")
    } else if let Some(dir) = rel.strip_suffix("/index.html") {
        format!("{url}/{dir}/")
    } else {
        format!("{url}/{rel}")
    }
}

/// Inject `<head>` metadata, then write the page to `staging/rel`.
fn write_html(
    staging: &Path,
    rel: &str,
    html: &str,
    config: &Config,
    description: &str,
    is_article: bool,
) -> Res<()> {
    let canonical = canonical_url(config, rel);
    let finalized = head::meta(html, config, &canonical, description, is_article);
    write_page(&staging.join(rel), &finalized)
}

/// Write `sitemap.xml` listing every output page.
fn write_sitemap(staging: &Path, config: &Config) -> Res<()> {
    let mut files = Vec::new();
    links::collect_html(staging, &mut files)?;
    files.sort();

    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for f in &files {
        let rel = f
            .strip_prefix(staging)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let loc = feed::xml(&canonical_url(config, &rel));
        s.push_str(&format!("  <url><loc>{loc}</loc></url>\n"));
    }
    s.push_str("</urlset>\n");
    fs::write(staging.join("sitemap.xml"), s)?;
    Ok(())
}

/// Write `robots.txt` pointing at the sitemap.
fn write_robots(staging: &Path, config: &Config) -> Res<()> {
    let url = config.url.trim_end_matches('/');
    fs::write(
        staging.join("robots.txt"),
        format!("User-agent: *\nAllow: /\n\nSitemap: {url}/sitemap.xml\n"),
    )?;
    Ok(())
}

/// Favicon files copied verbatim to the site root (and linked in `<head>`)
/// when present in `assets/`.
const FAVICONS: &[&str] = &["favicon.svg", "favicon.ico", "apple-touch-icon.png"];

/// Copy the stylesheet, fonts, and any favicons into the staging output.
fn copy_assets(root: &Path, staging: &Path) -> Res<()> {
    fs::copy(root.join("assets/style.css"), staging.join("style.css"))?;
    let fonts = root.join("assets/fonts");
    if fonts.is_dir() {
        copy_dir(&fonts, &staging.join("fonts"))?;
    }
    for icon in FAVICONS {
        let src = root.join("assets").join(icon);
        if src.is_file() {
            fs::copy(&src, staging.join(icon))?;
        }
    }
    Ok(())
}

/// Build the `<head>` favicon `<link>` tags for whichever icons exist in
/// `assets/`. An SVG (when present) is preferred by modern browsers; the
/// `.ico` stays as the universal fallback.
fn favicon_links(root: &Path) -> Vec<String> {
    let assets = root.join("assets");
    let mut links = Vec::new();
    if assets.join("favicon.svg").is_file() {
        links.push(r#"<link rel="icon" type="image/svg+xml" href="/favicon.svg">"#.to_string());
    }
    if assets.join("favicon.ico").is_file() {
        links.push(r#"<link rel="icon" href="/favicon.ico" sizes="any">"#.to_string());
    }
    if assets.join("apple-touch-icon.png").is_file() {
        links.push(r#"<link rel="apple-touch-icon" href="/apple-touch-icon.png">"#.to_string());
    }
    links
}

/// Read and parse `config.yaml`, with the path named in any error.
fn read_config(root: &Path) -> Res<Config> {
    let path = root.join("config.yaml");
    let text = fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let config =
        serde_yaml_ng::from_str(&text).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    Ok(config)
}

/// Replace `final_out` with `staging` using renames (same filesystem, fast).
/// Everything transient lives under the gitignored build dir.
fn swap_into_place(staging: &Path, final_out: &Path, build_dir: &Path) -> Res<()> {
    if final_out.exists() {
        let old = build_dir.join("_old");
        let _ = fs::remove_dir_all(&old);
        fs::rename(final_out, &old)?;
        fs::rename(staging, final_out)?;
        let _ = fs::remove_dir_all(&old);
    } else {
        fs::rename(staging, final_out)?;
    }
    Ok(())
}

fn write_page(path: &Path, html: &str) -> Res<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, html)?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Res<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}
