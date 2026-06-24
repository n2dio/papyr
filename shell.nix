# Dev shell for papyr — a Typst-powered static blog engine.
# papyr links Typst as a library and serves/watches itself, so the toolchain is
# just Rust — no separate typst/caddy/jq/python.
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    just # task runner for tool-dev tasks (see justfile)
  ];

  shellHook = ''
    echo "papyr dev shell"
    echo "  just build | release | install | check | fmt | lint"
    echo "  cargo run -- init <dir>   scaffold a site to test against"
  '';
}
