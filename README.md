# papyr

[![ci](https://github.com/n2dio/papyr/actions/workflows/ci.yml/badge.svg)](https://github.com/n2dio/papyr/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/papyr.svg)](https://crates.io/crates/papyr)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**A minimal static blog engine powered by [Typst](https://typst.app).** Write
posts in Typst instead of Markdown and get a fast, clean static site — out of a
single self-contained binary.

papyr links Typst in as a library and ships its own dev server and file watcher,
so there's no external toolchain to wrangle: no Node, no Python, no separate
Typst CLI, no static-server sidecar. Just `papyr`.

- **Zero client-side JS.** Code is syntax-highlighted and math is rendered to
  native MathML at build time — nothing ships to the browser but HTML and CSS.
- **Live reload.** `papyr serve` rebuilds on save and refreshes the open page.
- **Light + automatic dark mode**, with self-hosted fonts.
- **Posts, tags, an RSS feed, and standalone pages** out of the box.
- **One compile per file.** The HTML *and* the post's metadata come from the
  same compiled document via introspection — no double-compile, no shelling out.

## Quick start

```sh
cargo install papyr            # see Install for Nix / prebuilt binaries
papyr init my-blog && cd my-blog
papyr serve                    # http://localhost:8080, live-reloads on save
```

Edit a file under `posts/` and the page reloads itself. When you're happy,
`papyr build` writes the static site to `./site/`.

## Install

### Nix (flake)

```sh
nix run github:n2dio/papyr -- init my-blog      # run without installing
nix profile install github:n2dio/papyr          # install `papyr` into your profile
```

### Cargo

```sh
cargo install papyr                              # from crates.io
cargo install --git https://github.com/n2dio/papyr   # latest from git
```

Make sure `~/.cargo/bin` is on your `PATH`.

### Prebuilt binaries

Grab a tarball for your platform (Linux x86_64/aarch64 musl, macOS
x86_64/aarch64) from the [latest release](https://github.com/n2dio/papyr/releases/latest),
then extract `papyr` onto your `PATH`.

### From source

```sh
cargo install --path .     # optimized → ~/.cargo/bin/papyr   (or: just install)
```

While iterating, skip the slow optimized rebuild — `just install-fast` symlinks
the incrementally-built debug binary onto your PATH, so a plain `just build`
(~1s) updates `papyr` live. Run `just install` for the optimized binary when done.

## Usage

```sh
papyr init my-blog         # scaffold a new site
cd my-blog
papyr serve                # build, serve http://localhost:8080, rebuild + live-reload on change
papyr new my-post          # scaffold posts/my-post.typ
papyr build                # build the static site into ./site
papyr build --strict       # ...and fail if any internal link is broken (CI)
papyr clean                # remove build artifacts
```

Add `-v` / `--verbose` for per-file compile timings, HTTP request logs, and
watcher events.

## Writing a post

Create `posts/<slug>.typ` (the filename becomes `/posts/<slug>.html`):

```typ
#import "/lib/template.typ": post
#show: post.with(
  title: "My post title",
  date: "2026-01-01",            // date or full timestamp (see below)
  tags: ("typst", "web"),
  summary: "Shown on the home page and in the feed.",
)

Write prose in normal Typst markup. Inline `code`, fenced blocks, and math
(`$ E = m c^2 $`) all work.
```

`date` accepts either a plain date or a full timestamp, and drives ordering, the
RSS `pubDate`, and the `<time datetime>` attribute (only the date is shown):

```
2026-01-01                    # date only (midnight UTC)
2026-01-01 14:30:00           # date + time (UTC)
2026-01-01T14:30:00+02:00     # date + time with offset
```

`papyr new` stamps the current local time, so same-day posts order correctly.

Link between pages with the helpers (validated at build time):

```typ
#import "/lib/template.typ": post, post-link, page-link
See #post-link("other-post")[that post] or the #page-link("about")[about page].
```

[Typst Universe](https://typst.app/universe) packages work too — they're
downloaded on demand into the standard Typst cache:

```typ
#import "@preview/cetz:0.3.1": canvas   // fetched + cached on first build
```

Callout boxes for technical notes:

```typ
#import "/lib/template.typ": post, note, tip, warning
#note[Heads up.]  #tip[Do this.]  #warning[Careful.]
```

Set `toc: true` in a post's `show: post.with(...)` to get a collapsible table of
contents (headings always get hover anchors regardless).

## What every build produces (zero-config)

- **Prev/next** navigation between posts, **heading anchors**, and an opt-in **TOC**.
- **Social/SEO `<head>`**: OpenGraph + Twitter card, `description`, canonical URL,
  light/dark `theme-color`.
- **Discovery**: `sitemap.xml`, `robots.txt`, and RSS feed autodiscovery.
- **Print**: an ink-friendly `@media print` stylesheet for clean PDFs.
- A broken-internal-link check (fail the build with `--strict`).

## How a site is laid out

`papyr init` scaffolds these; edit them freely (each site owns its copy):

| Path | Purpose |
|------|---------|
| `config.yaml` | Site title, tagline, author, URL, description. Optional: a `nav:` list of `{label, href}` for the header, and `imprint:` (`true` → `/imprint.html`, or a path) for a footer imprint link. |
| `posts/*.typ`, `pages/*.typ` | Content. |
| `lib/template.typ` | Shared `post`/`page` show-rules, nav + footer, link helpers. |
| `gen/*.typ` | Index, tag, and tags-index listing pages. |
| `assets/` | `style.css`, the dark `code-theme.tmTheme`, and self-hosted fonts. |

## Developing papyr

This repo is the engine. The embedded scaffold lives in `lib/`, `gen/`, and
`assets/` (baked into the binary via `include_str!`/`include_bytes!`); the CLI
and build logic are in `src/`.

A dev shell with the full toolchain (cargo, rustc, rustfmt, clippy, just) is
provided via Nix — `nix develop` (flake) or `nix-shell` (legacy). With `direnv`
it loads automatically on `cd` (run `direnv allow` once).

```sh
just build | release | install | install-fast | test | check | fmt | lint
```

Notes: Typst's HTML export is still officially experimental; this pins Typst
`0.15` and upgrades deliberately. Math is native MathML (Typst 0.15). Code
blocks are dark in both themes because Typst bakes syntax colors in at build
time.

## License

[MIT](LICENSE) © Tim Eggert.
