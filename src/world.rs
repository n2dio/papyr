//! Typst `World` implementation backed by the project directory on disk.

use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Mutex;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_kit::downloader::Downloader;
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

/// Downloads Typst Universe packages over HTTPS using ureq (rustls) — avoids a
/// native openssl dependency. Resolved/cached by `SystemPackages`.
struct UreqDownloader {
    agent: ureq::Agent,
}

impl UreqDownloader {
    fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .user_agent(concat!("papyr/", env!("CARGO_PKG_VERSION")))
            .build();
        UreqDownloader {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Downloader for UreqDownloader {
    fn stream(&self, _key: &dyn Any, url: &str) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        let not_found = || io::Error::new(io::ErrorKind::NotFound, "404 Not Found");
        match self.agent.get(url).call() {
            Ok(resp) if resp.status().as_u16() == 404 => Err(not_found()),
            Ok(resp) => {
                let hint = resp
                    .headers()
                    .get(ureq::http::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<usize>().ok());
                Ok((hint, Box::new(resp.into_body().into_reader())))
            }
            Err(ureq::Error::StatusCode(404)) => Err(not_found()),
            Err(e) => Err(io::Error::other(e)),
        }
    }
}

/// Expensive, reusable state shared across every per-file compile in one build:
/// the loaded fonts, the project root, and a read cache. Recreated per build, so
/// the cache is naturally invalidated between builds (and within a build the
/// source files don't change underneath us).
pub struct Shared {
    pub root: PathBuf,
    fonts: FontStore,
    packages: SystemPackages,
    sources: Mutex<HashMap<FileId, Source>>,
    bytes: Mutex<HashMap<FileId, Bytes>>,
}

impl Shared {
    pub fn new(root: PathBuf) -> Self {
        let mut fonts = FontStore::new();
        // Embedded fonts only: deterministic, no system scan needed. HTML text
        // is emitted as markup; fonts matter only for math (now native MathML)
        // and any embedded frames.
        fonts.extend(typst_kit::fonts::embedded());
        Shared {
            root,
            fonts,
            // Uses the standard Typst package cache; downloads from the Universe
            // on demand (only if a post imports a package).
            packages: SystemPackages::new(UreqDownloader::new()),
            sources: Mutex::new(HashMap::new()),
            bytes: Mutex::new(HashMap::new()),
        }
    }
}

/// A `World` for compiling a single main file. Cheap to construct per file —
/// it borrows the shared state and only builds a fresh `Library`.
pub struct SiteWorld<'a> {
    shared: &'a Shared,
    main: FileId,
    library: LazyHash<Library>,
}

impl<'a> SiteWorld<'a> {
    pub fn new(shared: &'a Shared, main_rel: &str, inputs: Option<Dict>) -> Result<Self, String> {
        let mut builder = Library::builder().with_features([Feature::Html].into_iter().collect());
        if let Some(dict) = inputs {
            builder = builder.with_inputs(dict);
        }
        let vpath =
            VirtualPath::new(main_rel).map_err(|e| format!("invalid path {main_rel:?}: {e}"))?;
        let main = FileId::new(RootedPath::new(VirtualRoot::Project, vpath));
        Ok(SiteWorld {
            shared,
            main,
            library: LazyHash::new(builder.build()),
        })
    }
}

impl World for SiteWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.shared.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(source) = self.shared.sources.lock().unwrap().get(&id) {
            return Ok(source.clone());
        }
        let bytes = self.file(id)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| FileError::InvalidUtf8)?
            .to_string();
        let source = Source::new(id, text);
        self.shared
            .sources
            .lock()
            .unwrap()
            .insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.shared.bytes.lock().unwrap().get(&id) {
            return Ok(bytes.clone());
        }
        // Package files (@namespace/name:version) are fetched into the Typst
        // package cache and read from there; project files come from `root`.
        let path = match id.root() {
            VirtualRoot::Project => id
                .vpath()
                .realize(&self.shared.root)
                .map_err(|_| FileError::AccessDenied)?,
            VirtualRoot::Package(spec) => {
                let pkg = self
                    .shared
                    .packages
                    .obtain(spec)
                    .map_err(FileError::Package)?;
                id.vpath()
                    .realize(pkg.path())
                    .map_err(|_| FileError::AccessDenied)?
            }
        };
        let data = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
        let bytes = Bytes::new(data);
        self.shared.bytes.lock().unwrap().insert(id, bytes.clone());
        Ok(bytes)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.shared.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

impl typst_kit::diagnostics::DiagnosticWorld for SiteWorld<'_> {
    fn name(&self, id: FileId) -> String {
        id.vpath().get_without_slash().to_string()
    }
}
