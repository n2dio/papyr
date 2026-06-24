//! papyr — a minimal, self-contained, Typst-powered static blog generator.
//!
//! The tool is generic and site-agnostic: all site identity lives in
//! `config.yaml`, content in `posts/` + `pages/`, and look in `lib/` + `assets/`.

mod build;
mod dates;
mod feed;
mod head;
mod links;
mod model;
mod render;
mod scaffold;
mod serve;
mod text;
mod toc;
mod world;

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

/// Crate-wide fallible result with a boxed error.
pub(crate) type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(
    name = "papyr",
    about = "A minimal, Typst-powered static blog generator"
)]
struct Cli {
    /// Project root directory.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    /// Verbose logging (per-file timings, HTTP requests, watcher events).
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a fresh papyr site in a directory (default: current).
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Build the static site into ./site.
    Build {
        /// Fail the build if any internal link is broken.
        #[arg(long)]
        strict: bool,
    },
    /// Build, serve ./site, and rebuild on change.
    Serve {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Scaffold a new post: `papyr new my-slug`.
    New { slug: String },
    /// Remove build artifacts (site/, build/).
    Clean,
}

fn main() {
    let cli = Cli::parse();
    init_logging(cli.verbose);
    let result = match cli.command {
        Command::Init { path } => scaffold::init(&path),
        Command::Build { strict } => build::build(&cli.root, strict),
        Command::Serve { port } => serve::serve(&cli.root, port),
        Command::New { slug } => new_post(&cli.root, &slug),
        Command::Clean => clean(&cli.root),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn new_post(root: &Path, slug: &str) -> Res<()> {
    let path = root.join("posts").join(format!("{slug}.typ"));
    if path.exists() {
        return Err(format!("{} already exists", path.display()).into());
    }
    let content = format!(
        "#import \"/lib/template.typ\": post\n\
         #show: post.with(\n  \
         title: \"{slug}\",\n  \
         date: \"{}\",\n  \
         tags: (),\n  \
         summary: \"\",\n\
         )\n\n\
         Write your post here.\n",
        dates::now_timestamp()
    );
    fs::write(&path, content)?;
    println!("✓ created {}", path.display());
    Ok(())
}

/// Install a tracing subscriber when `--verbose` is set (or `RUST_LOG` is
/// present). Without it, `debug!`/request logs are no-ops and output stays clean.
fn init_logging(verbose: bool) {
    use tracing_subscriber::EnvFilter;
    let env = std::env::var_os("RUST_LOG");
    if !verbose && env.is_none() {
        return;
    }
    let filter = if env.is_some() {
        EnvFilter::from_default_env()
    } else {
        EnvFilter::new("info,papyr=debug,tower_http=debug")
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .init();
}

fn clean(root: &Path) -> Res<()> {
    let _ = fs::remove_dir_all(root.join("site"));
    let _ = fs::remove_dir_all(root.join("build"));
    println!("✓ cleaned");
    Ok(())
}
