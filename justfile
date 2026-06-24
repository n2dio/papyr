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
