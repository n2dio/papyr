#import "/lib/template.typ": post, note
#show: post.with(
  title: "Hello, papyr",
  date: "{date}",
  tags: ("intro",),
  summary: "Your first post — written in Typst, built by papyr.",
  toc: true,
  line-numbers: true,
)

Welcome to your new blog. Posts are written in #link("https://typst.app")[Typst]
and compiled to static HTML by papyr — no Markdown, no JavaScript.

#note[Edit `config.yaml` to set your title and links, then drop `.typ` files in `posts/`.]

= Code

Fenced blocks are highlighted at build time:

```rust
fn main() {
    println!("hello, papyr");
}
```

= Math

Inline like $e^(i pi) + 1 = 0$, or as a block:

$ sum_(k=1)^n k = (n (n + 1)) / 2 $

Edit `config.yaml`, add posts under `posts/`, then run `papyr serve`.
