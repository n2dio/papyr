{
  description = "papyr — a minimal, Typst-powered static blog generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        # Single source of truth for the version — read from Cargo.toml so it
        # never drifts from the crate metadata.
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        papyr = pkgs.rustPlatform.buildRustPackage {
          pname = "papyr";
          version = cargoToml.package.version;
          src = ./.;
          # Reproducible: resolve from the committed lockfile, no vendor hash to
          # keep in sync.
          cargoLock.lockFile = ./Cargo.lock;
          # The scaffold build is fully offline (embedded fonts, no Typst
          # Universe imports), so the integration tests run in the sandbox.
          # No native build inputs: ureq is rustls-based and fonts are embedded.
          meta = with pkgs.lib; {
            description = cargoToml.package.description;
            homepage = "https://github.com/n2dio/papyr";
            license = licenses.mit;
            mainProgram = "papyr";
          };
        };
      in
      {
        packages.default = papyr;
        packages.papyr = papyr;

        apps.default = {
          type = "app";
          program = "${papyr}/bin/papyr";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            just
          ];
          shellHook = ''
            echo "papyr dev shell (flake)"
            echo "  just build | release | install | check | fmt | lint"
            echo "  cargo run -- init <dir>   scaffold a site to test against"
          '';
        };
      });
}
