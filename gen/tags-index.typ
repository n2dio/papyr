// /tags/ landing page: every tag with a post count.
#import "/lib/template.typ": page, slugify

#let posts = json("/build/posts.json")
// Group by slug; keep the first raw tag seen as the display name.
#let counts = (:)
#let display = (:)
#for p in posts {
  for t in p.tags {
    let s = slugify(t)
    if s != "" {
      counts.insert(s, counts.at(s, default: 0) + 1)
      if s not in display {
        display.insert(s, t)
      }
    }
  }
}
#let slugs = counts.keys().sorted()

#show: page.with(title: "Tags")

#html.elem("h1")[Tags]

#html.elem(
  "ul",
  attrs: (class: "tag-cloud"),
  {
    for slug in slugs {
      html.elem(
        "li",
        html.elem(
          "a",
          attrs: (href: "/tags/" + slug + ".html", class: "tag"),
          [#("#" + display.at(slug)) #html.elem("span", attrs: (class: "count"))[#counts.at(slug)]],
        ),
      )
    }
  },
)
