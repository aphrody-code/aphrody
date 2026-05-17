# SPDX-License-Identifier: Apache-2.0
{
  description = "aphrody — cross-platform Rust CLI with embedded AGNTCY a2a/v0.4 manifest";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          targets = [
            "wasm32-unknown-unknown"
            "wasm32-wasip1"
          ];
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Derive version from Cargo.lock or hard-pin to canary channel.
        version = "1.0.0-canary";
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "aphrody";
          inherit version;

          src = ../..;  # repo root — flake lives at packaging/nix/

          cargoLock = {
            lockFile = ../../Cargo.lock;
          };

          # pkg-config locates openssl at build time; the library itself is
          # a runtime dep only on the host where the binary runs (static
          # linking is not forced here so the derivation stays composable
          # with NixOS's dynamic linker).
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.openssl
          ];

          # Build only the CLI binary — skip the workspace members that
          # require a running X session or GPU (crates/gui) or that are
          # tooling-only (mrx-cli, aphrody-translate …).
          buildFeatures = [];
          cargoBuildFlags = [ "-p" "aphrody" ];

          # Integration tests need network access and a real filesystem
          # layout that sandbox mode does not provide.  Unit tests run in
          # CI (`cargo ci-offline && cargo nextest run`).
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Cross-platform Rust CLI with embedded AGNTCY a2a/v0.4 manifest";
            homepage = "https://github.com/aphrody-code/aphrody";
            license = licenses.asl20;
            maintainers = [
              {
                name = "aphrody-code";
                email = "noreply@aphrody-code.dev";
                github = "aphrody-code";
              }
            ];
            platforms = platforms.unix;
            mainProgram = "aphrody";
          };
        };

        # `nix develop` drops you into a shell with the full Rust nightly
        # toolchain, Bun, and all CI quality-gate tools pre-installed.
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.bun
            pkgs.cargo-nextest
            pkgs.cargo-deny
            pkgs.cargo-zigbuild
            pkgs.pkg-config
            pkgs.openssl
          ];

          # Surface the nightly toolchain name so cargo-toolchain-file
          # overrides work correctly inside the shell.
          RUSTUP_TOOLCHAIN = "nightly";
        };

        # Expose the toolchain derivation for downstream flakes that want
        # to share the exact same nightly pin (e.g. a CI overlay).
        packages.rustToolchain = rustToolchain;
      });
}
