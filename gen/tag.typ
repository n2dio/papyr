// One page per tag. papyr passes the slug + display name via `--input`.
#import "/lib/template.typ": page, format-tags, slugify, date-el

#let slug = sys.inputs.at("slug")
#let name = sys.inputs.at("name")
#let posts = json("/build/posts.json").filter(p => p.tags.map(slugify).contains(slug))

#show: page.with(title: "#" + name)

#html.elem("h1", attrs: (class: "tag-title"))[Tagged #("#" + name)]

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
