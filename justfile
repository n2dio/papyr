# Tasks for developing the `papyr` engine itself (the Typst blog tool in src/).
# Run the blog through the binary, e.g. `cargo run -- serve` (dev) or
# `./target/release/papyr serve` after `just release`.

default:
    @just --list

# debug build of the papyr binary
build:
    cargo build

# optimized release binary at ./target/release/papyr
release:
    cargo build --release

# install the optimized papyr binary into ~/.cargo/bin (must be on your PATH)
install:
    cargo install --path .

# fast install for iterating: symlink the incrementally-built debug binary onto
# PATH. After this, a plain `just build` (or `cargo build`) updates papyr live —
# no recompiling the dependency tree in release. Run `just install` for the
# optimized binary when you're done testing.
install-fast: build
    mkdir -p ~/.cargo/bin
    ln -sf "{{justfile_directory()}}/target/debug/papyr" ~/.cargo/bin/papyr
    @echo "linked ~/.cargo/bin/papyr → target/debug/papyr (update it with: just build)"

# run the test suite
test:
    cargo test

# type-check without building a binary
check:
    cargo check

# format the Rust sources
fmt:
    cargo fmt

# lint with clippy (warnings are errors)
lint:
    cargo clippy -- -D warnings

# Cut a release: bump the version, sync Cargo.lock, commit, tag, and push.
# The pushed tag triggers release.yml (prebuilt binaries) and publish.yml
# (crates.io). Run from a clean `main`.   Usage: just tag 0.1.3
tag version:
    #!/usr/bin/env bash
    set -euo pipefail
    test -z "$(git status --porcelain)" || { echo "✗ working tree not clean — commit or stash first" >&2; exit 1; }
    if git rev-parse -q --verify "refs/tags/v{{version}}" >/dev/null; then
        echo "✗ tag v{{version}} already exists" >&2; exit 1
    fi
    # Bump the [package] version (the first `version = "…"` line only).
    perl -i -pe 'if (!$d && s/^version = ".*"/version = "{{version}}"/) { $d = 1 }' Cargo.toml
    # Sync the papyr entry in Cargo.lock to the new version (deps untouched).
    # THIS is the step a bare `git tag` skips — without it Cargo.lock still
    # records the old version and `cargo publish --locked` refuses to proceed.
    cargo update -p papyr --precise "{{version}}"
    # Prove Cargo.toml and Cargo.lock agree (the exact check publish.yml runs).
    cargo check --locked
    git add Cargo.toml Cargo.lock
    git commit -m "Release v{{version}}"
    git tag -a "v{{version}}" -m "v{{version}}"
    git push origin HEAD --follow-tags
    echo "✓ pushed v{{version}} — release.yml + publish.yml will run on the tag"
