//! The `serve` command: build, serve `site/` with axum, rebuild on change.

use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

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

    loop {
        // Block until something changes, then debounce a short burst.
        let _ = rx.recv()?;
        std::thread::sleep(Duration::from_millis(200));
        while rx.try_recv().is_ok() {}

        println!("› change detected — rebuilding");
        if let Err(e) = build::build(root, false) {
            eprintln!("build error: {e}");
        }
    }
}
