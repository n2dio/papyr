# Live reload for `papyr serve`

## Goal

When `papyr serve` rebuilds the site on a source change, connected browsers
reload automatically instead of requiring a manual refresh.

## Background

`papyr serve` already:

- Builds `site/`, then serves it with axum (`ServeDir`).
- Runs a `notify` watcher thread that rebuilds on source changes, with
  mtime-gating to avoid a rebuild loop from the build's own output churn.

What's missing is the browser-facing half: nothing tells the page a rebuild
happened.

## Decisions

- **Transport:** Server-Sent Events (SSE). One-directional server→browser is all
  a reload needs; axum 0.8 has built-in SSE support, no new dependencies.
- **Client injection:** on-the-fly at serve time via middleware, so the static
  `site/` output stays clean (no live-reload code ships to production).
- **Reload behaviour:** full `location.reload()`.

## Architecture

Three pieces wired through one `tokio::sync::broadcast::Sender<()>` created in
`serve()`:

1. **Watcher → signal.** `watch_loop` gains a `reload_tx: broadcast::Sender<()>`
   parameter. After a *successful* rebuild it calls `reload_tx.send(())`. A
   failed build sends nothing, so the browser keeps the last good page.
   `broadcast::Sender::send` is non-async, so it works from the existing std
   thread.

2. **SSE route** `GET /__papyr/livereload`. Returns an
   `axum::response::sse::Sse` stream; each broadcast tick emits one event
   (`data: reload`). Keep-alive pings hold the connection open through idle
   periods.

3. **HTML-injection middleware** (`axum::middleware::from_fn`) wrapping the
   `ServeDir` fallback. For responses with `content-type: text/html`, it buffers
   the body and inserts the client `<script>` before `</body>` (appends if no
   `</body>` present). Non-HTML responses pass through untouched.

## Injected client

A static string constant, injected into each served HTML page:

```html
<script>
  new EventSource("/__papyr/livereload").onmessage = () => location.reload();
</script>
```

`EventSource` auto-reconnects, so when the server restarts the page reconnects
and reloads on the next signal.

## Reserved path

`/__papyr/livereload`. The `__papyr` prefix won't collide with generated site
content.

## Error handling

- Build failures already print to stderr and send no reload signal.
- `broadcast::Sender::send` returning `Err` (no subscribers) is ignored.
- Body buffering is bounded to `text/html` responses only.

## Testing

- Unit test for the pure HTML-injection function: injects before `</body>`;
  appends when `</body>` is absent; leaves non-HTML bodies unchanged.
- End-to-end SSE flow verified manually via `papyr serve`.
