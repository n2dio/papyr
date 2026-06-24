//! End-to-end tests driving the real `papyr` binary: scaffold a site, build it,
//! and check the output. Uses CARGO_BIN_EXE_papyr (the built binary) and
//! CARGO_TARGET_TMPDIR (a per-run temp dir under target/) — no external /tmp.

use std::path::{Path, PathBuf};
use std::process::Command;

fn papyr() -> &'static str {
    env!("CARGO_BIN_EXE_papyr")
}

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn init_then_build_produces_a_site() {
    let site = fresh("init-build");

    let init = Command::new(papyr())
        .args(["init", site.to_str().unwrap()])
        .status()
        .expect("run papyr init");
    assert!(init.success(), "init failed");

    let build = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .status()
        .expect("run papyr build");
    assert!(build.success(), "build failed");

    let out = site.join("site");
    for f in [
        "index.html",
        "posts/hello-papyr.html",
        "about.html",
        "imprint.html",
        "feed.xml",
        "style.css",
        "tags/index.html",
        "tags/intro.html", // the example post is tagged "intro"
        "fonts/jetbrains-mono-400.woff2",
    ] {
        assert!(out.join(f).is_file(), "missing output: {f}");
    }
}

#[test]
fn tag_slugs_agree_between_rust_and_typst() {
    let site = fresh("tag-slugs");

    Command::new(papyr())
        .args(["init", site.to_str().unwrap()])
        .status()
        .expect("init");

    // A post whose tags need slugification on both sides.
    let post = "#import \"/lib/template.typ\": post\n\
        #show: post.with(title: \"Tricky\", date: \"2026-01-02\", \
        tags: (\"Machine Learning\", \"C++\"), summary: \"s\")\n\nBody.\n";
    std::fs::write(site.join("posts/tricky.typ"), post).unwrap();

    let output = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .output()
        .expect("build");
    assert!(output.status.success(), "build failed");

    // If the Rust and Typst slugifiers disagreed, the link checker would print
    // a broken-link warning to stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("broken link"),
        "broken links reported (slugify mismatch?):\n{stderr}"
    );

    let out = site.join("site");
    assert!(out.join("tags/machine-learning.html").is_file());
    assert!(out.join("tags/c.html").is_file());
}

#[test]
fn build_does_not_panic_on_a_bad_date() {
    let site = fresh("bad-date");

    Command::new(papyr())
        .args(["init", site.to_str().unwrap()])
        .status()
        .expect("init");

    let post = "#import \"/lib/template.typ\": post\n\
        #show: post.with(title: \"Oops\", date: \"2026-13-99\", tags: (), summary: \"s\")\n\nBody.\n";
    std::fs::write(site.join("posts/oops.typ"), post).unwrap();

    let output = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .output()
        .expect("build");

    // Build still succeeds, warns about the date, and the feed contains the raw
    // (un-converted) date rather than crashing.
    assert!(
        output.status.success(),
        "build should not fail on a bad date"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is not a valid date"),
        "expected a date warning:\n{stderr}"
    );
    let feed = std::fs::read_to_string(site.join("site/feed.xml")).unwrap();
    assert!(feed.contains("2026-13-99"));
}

#[test]
fn strict_fails_on_a_broken_link() {
    let site = fresh("strict");

    Command::new(papyr())
        .args(["init", site.to_str().unwrap()])
        .status()
        .expect("init");

    // A post linking to a post that doesn't exist.
    let post = "#import \"/lib/template.typ\": post, post-link\n\
        #show: post.with(title: \"L\", date: \"2026-01-01\", tags: (), summary: \"s\")\n\n\
        See #post-link(\"does-not-exist\")[this].\n";
    std::fs::write(site.join("posts/linker.typ"), post).unwrap();

    // Non-strict build warns but succeeds.
    let ok = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .status()
        .expect("build");
    assert!(ok.success());

    // --strict turns the broken link into a failure.
    let strict = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build", "--strict"])
        .status()
        .expect("build --strict");
    assert!(
        !strict.success(),
        "strict build should fail on a broken link"
    );
}

#[test]
fn failed_rebuild_keeps_previous_site() {
    let site = fresh("failed-rebuild");

    Command::new(papyr())
        .args(["init", site.to_str().unwrap()])
        .status()
        .expect("init");

    // First build succeeds.
    let ok = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .status()
        .expect("build");
    assert!(ok.success());
    let index = site.join("site/index.html");
    let before = std::fs::read_to_string(&index).expect("first build wrote index.html");

    // A post that fails to compile should make the rebuild fail...
    let broken = "#import \"/lib/template.typ\": post\n\
        #show: post.with(title: \"X\", date: \"2026-01-01\", tags: (), summary: \"s\")\n\n\
        #this_variable_is_undefined\n";
    std::fs::write(site.join("posts/broken.typ"), broken).unwrap();
    let status = Command::new(papyr())
        .args(["--root", site.to_str().unwrap(), "build"])
        .status()
        .expect("build");
    assert!(!status.success(), "build should fail on a broken post");

    // ...without destroying the previously built site.
    assert_eq!(
        std::fs::read_to_string(&index).expect("previous index.html should survive"),
        before,
        "a failed rebuild must leave site/ untouched"
    );
}
