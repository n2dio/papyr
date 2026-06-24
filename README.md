# papyr

A minimal, self-contained static **blog engine** — write posts in
[Typst](https://typst.app) (not Markdown), get a clean static site. papyr is a
single Rust binary that links Typst as a library: it builds, serves, and
watches, with no Node, no Python, and no separate `typst`/Caddy.

- Syntax-highlighted code and math rendered **at build time** (code →
  inline-styled HTML, math → native MathML) — pages ship zero client-side JS.
- Light theme with automatic dark mode; self-hosted fonts.
- Posts, tags, an RSS feed, and standalone pages.
- One in-process compile per file: the HTML *and* the post metadata come from
  the same compiled document (via introspection) — no double-compile, no
  shelling out.

## Install

```sh
cargo install --path .     # optimized → ~/.cargo/bin/papyr   (or: just install)
```

Make sure `~/.cargo/bin` is on your `PATH`.

While iterating, skip the slow optimized rebuild — `just install-fast` symlinks
the incrementally-built debug binary onto your PATH, so a plain `just build`
(~1s) updates `papyr` live. Run `just install` for the optimized binary when done.

## Use

```sh
papyr init my-blog         # scaffold a new site
cd my-blog
papyr serve                # build, serve http://localhost:8080, rebuild on change
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

```sh
just build | release | install | install-fast | test | check | fmt | lint
```

Notes: Typst's HTML export is still officially experimental; this pins Typst
`0.15` and upgrades deliberately. Math is native MathML (Typst 0.15). Code
blocks are dark in both themes because Typst bakes syntax colors in at build
time.

## License

MIT OR Apache-2.0.
