// Homepage: reverse-chronological post list (reads build/posts.json).
#import "/lib/template.typ": page, site, format-tags, date-el

#let posts = json("/build/posts.json")

#show: page.with(title: none)

#html.elem(
  "ul",
  attrs: (class: "post-list"),
  {
    for p in posts {
      html.elem(
        "li",
        {
          html.elem(
            "h2",
            attrs: (class: "post-link"),
            html.elem("a", attrs: (href: "/posts/" + p.slug + ".html"))[#p.title],
          )
          html.elem(
            "p",
            attrs: (class: "post-meta"),
            {
              date-el(p.date)
              if p.tags.len() > 0 [ · #format-tags(p.tags)]
            },
          )
          html.elem("p", attrs: (class: "summary"))[#p.summary]
        },
      )
    }
  },
)
