// Shared theme for a papyr site.
// Every post does:  #show: post.with(title: ..., date: ..., tags: (...), summary: ...)
// Pages use:        #show: page.with(title: ...)

#let site = yaml("/config.yaml")

// Turn an arbitrary tag into a URL/filename-safe slug. MUST match the Rust
// `slugify` in build.rs (the build-time link checker verifies they agree).
#let slugify(s) = lower(s).replace(regex("[^a-z0-9]+"), "-").trim("-")

// --- internal links ----------------------------------------------------------
// Link to another post by slug, or a page by name, so URLs live in one place.
// Usage:  #post-link("hello-world")[my first post]  ·  #page-link("about")[about]
// `papyr build` warns if any of these point at a missing page.
#let post-link(slug, body) = link("/posts/" + slug + ".html", body)
#let page-link(name, body) = link("/" + name + ".html", body)

// Render a post date/timestamp as a <time> element: the full value stays in the
// machine-readable `datetime` attribute, only the date part is shown.
#let date-el(d) = html.elem("time", attrs: (datetime: d))[#d.split("T").at(0).split(" ").at(0)]

// Admonition callouts. Usage in a post: #note[...], #tip[...], #warning[...]
#let callout(kind, body) = html.elem("div", attrs: (class: "callout callout-" + kind), body)
#let note(body) = callout("note", body)
#let tip(body) = callout("tip", body)
#let warning(body) = callout("warning", body)

// --- code highlighting -------------------------------------------------------
// Dark code surface in both light & dark mode (syntect colors are baked in at
// build time and can't switch per prefers-color-scheme, so we fix them dark).
#let with-code-theme(body) = {
  set raw(theme: "/assets/code-theme.tmTheme")
  body
}

// --- chrome ------------------------------------------------------------------
#let nav-link(href, label) = html.elem("a", attrs: (href: href))[#label]

#let site-header() = html.elem(
  "header",
  attrs: (class: "site-header"),
  {
    html.elem(
      "div",
      attrs: (class: "masthead"),
      {
        html.elem("a", attrs: (href: "/", class: "brand"))[#site.title]
        html.elem("div", attrs: (class: "brand-tagline"))[#site.tagline]
      },
    )
    html.elem(
      "nav",
      {
        // Override by adding a `nav:` list of {label, href} to config.yaml.
        let default-nav = (
          (label: "Posts", href: "/"),
          (label: "Tags", href: "/tags/"),
          (label: "About", href: "/about.html"),
          (label: "RSS", href: "/feed.xml"),
        )
        for item in site.at("nav", default: default-nav) {
          nav-link(item.href, item.label)
        }
      },
    )
  },
)

// Imprint link is opt-in: set `imprint: true` (→ /imprint.html) or
// `imprint: "/some/path.html"` in config.yaml. Absent/false → no link.
#let imprint-href = {
  let v = site.at("imprint", default: none)
  if v == true { "/imprint.html" } else if type(v) == str { v } else { none }
}

#let site-footer() = html.elem(
  "footer",
  attrs: (class: "site-footer"),
  html.elem("p", {
    [© #site.author · ]
    if imprint-href != none {
      html.elem("a", attrs: (href: imprint-href))[Imprint]
      [ · ]
    }
    [built with ]
    html.elem("a", attrs: (href: "https://typst.app"))[Typst]
    [ and ]
    html.elem("a", attrs: (href: "https://github.com/n2dio/papyr"))[papyr]
  }),
)

// Floating "back to top" link. `#top` is the spec's special top-of-document
// fragment (no target element needed); CSS reveals it on scroll, zero JS.
#let to-top() = html.elem(
  "a",
  attrs: (href: "#top", class: "to-top", "aria-label": "Back to top"),
  [↑],
)

// Generic page shell (used by About and the generated index/tag pages).
#let page(title: none, body) = {
  set document(title: if title == none { site.title } else { title + " — " + site.title })
  show: with-code-theme
  site-header()
  html.elem("main", attrs: (class: "container"), body)
  site-footer()
  to-top()
}

// --- post --------------------------------------------------------------------
#let format-tags(tags) = html.elem(
  "span",
  attrs: (class: "tags"),
  {
    for t in tags {
      let s = slugify(t)
      if s != "" {
        html.elem("a", attrs: (href: "/tags/" + s + ".html", class: "tag"))[\##t]
      }
    }
  },
)

#let post(
  title: "",
  date: "",
  tags: (),
  summary: "",
  toc: false,
  collapsed: false,
  line-numbers: false,
  body,
) = {
  // Frontmatter that papyr reads back via document introspection.
  [#metadata((
      title: title,
      date: date,
      tags: tags,
      summary: summary,
      toc: toc,
      collapsed: collapsed,
      line-numbers: line-numbers,
    )) <frontmatter>]

  set document(title: title + " — " + site.title)
  show: with-code-theme

  site-header()
  html.elem(
    "main",
    attrs: (class: "container"),
    html.elem(
      "article",
      {
        html.elem("h1", attrs: (class: "post-title"))[#title]
        html.elem(
          "p",
          attrs: (class: "post-meta"),
          {
            date-el(date)
            if tags.len() > 0 [ · #format-tags(tags)]
          },
        )
        body
      },
    ),
  )
  site-footer()
  to-top()
}
