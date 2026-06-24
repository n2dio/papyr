//! The `serve` command: build, serve `site/` with axum, rebuild on change.

use std::error::Error;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use axum::Router;
use notify::{RecursiveMode, Watcher};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::{build, Res};

pub fn serve(root: &Path, port: u16) -> Res<()> {
    build::build(root, false)?;

    // Watch sources in a background thread and rebuild on change.
    let watch_root = root.to_path_buf();
    std::thread::spawn(move || {
        if let Err(e) = watch_loop(&watch_root) {
            eprintln!("watch error: {e}");
        }
    });

    // Serve site/ with axum. All internal links use explicit .html or
    // directory paths, so ServeDir + index-on-directories is enough.
    let site = root.join("site");
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let app = Router::new()
            .fallback_service(ServeDir::new(&site).append_index_html_on_directories(true))
            .layer(TraceLayer::new_for_http());
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("› serving http://{addr}  (ctrl-c to stop)");
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn Error>>(())
    })
}

fn watch_loop(root: &Path) -> Res<()> {
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
        if let Err(e) = build::build(root, false) {
            eprintln!("build error: {e}");
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
        if meta.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    visit(&entry.path(), latest);
                }
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
