//! The `serve` command: build, serve `site/` with axum, rebuild on change.
//!
//! In addition to rebuilding, the server pushes a live-reload signal to
//! connected browsers over SSE (`/__papyr/livereload`) after each successful
//! rebuild. A small client script is injected into served HTML on the fly, so
//! the static `site/` output stays clean.

use std::convert::Infallible;
use std::error::Error;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use notify::{RecursiveMode, Watcher};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::{Res, build};

/// Path the SSE live-reload endpoint is served from. The `__papyr` prefix
/// won't collide with generated site content.
const LIVERELOAD_PATH: &str = "/__papyr/livereload";

/// Client script injected into served HTML pages. `EventSource` auto-reconnects,
/// so when the server restarts the page reconnects and reloads on the next tick.
const LIVERELOAD_SCRIPT: &str =
    "<script>new EventSource(\"/__papyr/livereload\").onmessage=()=>location.reload()</script>";

/// Insert the live-reload script into an HTML document, just before `</body>`
/// (or appended if there is no closing body tag).
fn inject_livereload(html: &str) -> String {
    match html.rfind("</body>") {
        Some(idx) => {
            let mut out = String::with_capacity(html.len() + LIVERELOAD_SCRIPT.len());
            out.push_str(&html[..idx]);
            out.push_str(LIVERELOAD_SCRIPT);
            out.push_str(&html[idx..]);
            out
        }
        None => format!("{html}{LIVERELOAD_SCRIPT}"),
    }
}

pub fn serve(root: &Path, port: u16) -> Res<()> {
    build::build(root, false)?;

    // A successful rebuild broadcasts `()`; every connected SSE client receives
    // it and reloads. Buffer a few so a tick isn't lost between rebuild and the
    // handler polling. Subscribers are created per-connection in the handler.
    let (reload_tx, _) = broadcast::channel::<()>(16);

    // Watch sources in a background thread and rebuild on change.
    let watch_root = root.to_path_buf();
    let watch_tx = reload_tx.clone();
    std::thread::spawn(move || {
        if let Err(e) = watch_loop(&watch_root, watch_tx) {
            eprintln!("watch error: {e}");
        }
    });

    // Serve site/ with axum. All internal links use explicit .html or
    // directory paths, so ServeDir + index-on-directories is enough.
    let site = root.join("site");
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let sse_tx = reload_tx.clone();
        let app = Router::new()
            .route(LIVERELOAD_PATH, get(move || livereload(sse_tx.clone())))
            .fallback_service(ServeDir::new(&site).append_index_html_on_directories(true))
            // Inject the client script into HTML responses on the fly.
            .layer(middleware::from_fn(inject_middleware))
            .layer(TraceLayer::new_for_http());
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("› serving http://{addr}  (ctrl-c to stop, live reload on)");
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn Error>>(())
    })
}

/// SSE endpoint: emit a `reload` event for every broadcast tick. Keep-alive
/// pings hold the connection open through idle periods.
async fn livereload(
    tx: broadcast::Sender<()>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe())
        // A tick (or a lag notification) both mean "something changed" — reload.
        .map(|_| Ok(Event::default().data("reload")));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Middleware that injects the live-reload client into `text/html` responses.
/// Non-HTML responses pass through untouched.
async fn inject_middleware(req: Request, next: Next) -> Response {
    let resp = next.run(req).await;

    let is_html = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));
    if !is_html {
        return resp;
    }

    let (mut parts, body) = resp.into_parts();
    // HTML pages are small; cap the buffer to guard against anything unexpected.
    let bytes = match axum::body::to_bytes(body, 16 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return Response::from_parts(parts, Body::empty()),
    };
    let injected = inject_livereload(&String::from_utf8_lossy(&bytes));
    // Body length changed; drop the stale Content-Length so it's recomputed.
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(injected))
}

fn watch_loop(root: &Path, reload_tx: broadcast::Sender<()>) -> Res<()> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    for dir in ["posts", "pages", "lib", "gen", "assets"] {
        let p = root.join(dir);
        if p.exists() {
            watcher.watch(&p, RecursiveMode::Recursive)?;
        }
    }
    let config = root.join("config.yaml");
    if config.exists() {
        watcher.watch(&config, RecursiveMode::NonRecursive)?;
    }

    // A rebuild writes into build/ and site/ (not watched), but on macOS that
    // churn still surfaces as events here — so rebuilding on every event would
    // loop forever. Gate on a real source-mtime change: events that don't
    // correspond to an edited source (i.e. the build's own output) are ignored.
    let mut last_built = SystemTime::now();
    loop {
        // Block until something changes, then debounce a short burst.
        let _ = rx.recv()?;
        std::thread::sleep(Duration::from_millis(200));
        while rx.try_recv().is_ok() {}

        if latest_source_mtime(root) <= last_built {
            continue; // no source actually changed — build churn, ignore it
        }

        // Stamp the time *before* building so an edit made mid-build is still
        // seen as newer next time and isn't lost.
        let started = SystemTime::now();
        println!("› change detected — rebuilding");
        match build::build(root, false) {
            // Tell connected browsers to reload only after a clean rebuild, so a
            // failed build leaves the last good page in place.
            Ok(()) => {
                let _ = reload_tx.send(());
            }
            Err(e) => eprintln!("build error: {e}"),
        }
        last_built = started;
        while rx.try_recv().is_ok() {} // drop the events the build just produced
    }
}

/// Newest mtime across all watched sources (files and their directories, so
/// creates and removes register too). Lets the watcher tell a real edit apart
/// from the filesystem noise the build's own output produces.
fn latest_source_mtime(root: &Path) -> SystemTime {
    fn visit(path: &Path, latest: &mut SystemTime) {
        let Ok(meta) = fs::metadata(path) else { return };
        if let Ok(m) = meta.modified() {
            *latest = (*latest).max(m);
        }
        if meta.is_dir()
            && let Ok(entries) = fs::read_dir(path)
        {
            for entry in entries.flatten() {
                visit(&entry.path(), latest);
            }
        }
    }

    let mut latest = SystemTime::UNIX_EPOCH;
    for dir in ["posts", "pages", "lib", "gen", "assets"] {
        visit(&root.join(dir), &mut latest);
    }
    visit(&root.join("config.yaml"), &mut latest);
    latest
}

#[cfg(test)]
mod tests {
    use super::{LIVERELOAD_SCRIPT, inject_livereload};

    #[test]
    fn injects_before_closing_body() {
        let out = inject_livereload("<html><body>hi</body></html>");
        assert_eq!(
            out,
            format!("<html><body>hi{LIVERELOAD_SCRIPT}</body></html>")
        );
    }

    #[test]
    fn appends_when_no_body_tag() {
        let out = inject_livereload("<p>fragment</p>");
        assert_eq!(out, format!("<p>fragment</p>{LIVERELOAD_SCRIPT}"));
    }

    #[test]
    fn injects_before_last_body_when_multiple() {
        // Only the final </body> should be the insertion point.
        let out = inject_livereload("<body>a</body><body>b</body>");
        assert_eq!(
            out,
            format!("<body>a</body><body>b{LIVERELOAD_SCRIPT}</body>")
        );
    }
}
