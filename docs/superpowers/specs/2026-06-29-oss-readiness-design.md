# Make papyr OSS-ready

## Goal

Make the `papyr` repository easy to install, document, and contribute to as an
open-source project: clear install paths for Nix, Cargo (git + crates.io), and
prebuilt binaries; complete licensing and crate metadata; and CI.

## Background

Current state of the repo:

- `Cargo.toml` declares `license = "MIT OR Apache-2.0"` but **no LICENSE file
  exists**, and it lacks `repository`, `homepage`, `readme`, `authors`,
  `keywords`, and `categories` — all expected by crates.io / docs.rs. The
  license will be narrowed to **MIT only**.
- Only `.github/workflows/release.yml` exists (builds binary tarballs on `v*`
  tags for linux-musl + macOS). There is **no test/lint CI**.
- Nix support is a legacy `shell.nix` dev shell only — no `flake.nix`, so no
  `nix run` / installable package.
- The crate name `papyr` is **available** on crates.io (verified: 404).
- The scaffold build is fully offline — embedded fonts, no Typst Universe
  package imports — so tests run without network (relevant for the Nix sandbox).

## Decisions

- crates.io: prepare metadata + automate publishing on tag, but the maintainer
  pushes the first tag / adds the token. Publishing is irreversible (yank only).
- Nix: add a `flake.nix` (package + app + devShell); keep `shell.nix` for
  non-flake users.
- CI: add a `ci.yml` (test + clippy + fmt) on push/PR.
- Identity: copyright **Tim Eggert**, contact **tim@n2d.io**, year **2026**.
- This work lands on a fresh branch off `main` (not stacked on `live-reload`).

## Parts

### 1. Licensing

- Add a single `LICENSE` file (standard MIT text) at the repo root, copyright
  line `2026 Tim Eggert`.
- Change `Cargo.toml` `license` from `"MIT OR Apache-2.0"` to `"MIT"`.
- README License section reads `MIT` and points at `LICENSE`.

### 2. `Cargo.toml` metadata

Add:

```toml
authors = ["Tim Eggert <tim@n2d.io>"]
repository = "https://github.com/n2dio/papyr"
homepage = "https://github.com/n2dio/papyr"
readme = "README.md"
keywords = ["typst", "static-site-generator", "blog", "ssg", "cli"]
categories = ["command-line-utilities", "web-programming"]
```

`rust-version` is intentionally **omitted** — a wrong MSRV floor is worse than
none; a verified value can be added later with `cargo-msrv`.

### 3. Nix flake

`flake.nix` with three outputs, using `flake-utils` for multi-system
(`x86_64`/`aarch64` × linux/darwin):

- `packages.default` — `rustPlatform.buildRustPackage` with
  `cargoLock.lockFile = ./Cargo.lock` (reproducible, no vendored hash). Version
  read from `Cargo.toml` via `importTOML` so it never drifts. `doCheck` stays on
  (offline integration tests run during `nix build`). No native build deps
  (ureq is rustls-based; fonts are embedded).
- `apps.default` — enables `nix run github:n2dio/papyr -- init my-blog`.
- `devShells.default` — same toolchain as `shell.nix` (cargo, rustc, rustfmt,
  clippy, just).

`shell.nix` is kept. `.envrc` prefers the flake dev shell when available and
falls back to `shell.nix`.

### 4. crates.io publishing (automated)

`.github/workflows/publish.yml`, triggered on `v*` tags (same as `release.yml`):

- Runs `cargo publish --locked`.
- Gated on a `CRATES_IO_TOKEN` repo secret; if the secret is absent the job
  no-ops with a clear message, so nothing publishes until the maintainer adds
  the token and pushes a tag.

### 5. CI + docs

- `.github/workflows/ci.yml` — on push / PR to `main`: `cargo test`,
  `cargo clippy -- -D warnings`, `cargo fmt --check`.
- README install section reworked into clear paths:
  - **Nix** — `nix run`, `nix profile install github:n2dio/papyr`, flake dev
    shell.
  - **Cargo** — `cargo install papyr` (once published) and
    `cargo install --git https://github.com/n2dio/papyr` (works today).
  - **Prebuilt binaries** — the release tarballs already produced by
    `release.yml`.
  - Status badges: CI, crates.io version, license.

## Out of scope

- `CONTRIBUTING.md` (the `justfile` + dev shell already document the workflow).
- A verified MSRV / `rust-version` floor.
- Actually publishing v0.1.1 this round (the maintainer triggers it).

## Testing / verification

- `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` pass locally.
- `nix flake check` and `nix build` succeed (build + offline tests).
- `nix run .# -- --help` runs the built binary.
- `cargo publish --dry-run --locked` succeeds (metadata + packaging valid)
  without actually publishing.
