{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib mkShellNoCC rust-bin;

        rustupToolchain = (lib.importTOML ./rust-toolchain.toml).toolchain;
        crateMetadata = (lib.importTOML ./Cargo.toml).package;

        # Rust toolchain for development
        rust-dev = rust-bin.fromRustupToolchain rustupToolchain;
        rust-dev-with-rust-analyzer = rust-dev.override (prev: {
          extensions = prev.extensions ++ [
            "rust-src"
            "rust-analyzer"
          ];
        });

        # Rust toolchain of MSRV
        rust-msrv = rust-bin.fromRustupToolchain {
          channel = crateMetadata.rust-version;
          profile = "minimal";
        };

        mkDevShell =
          devPkgs:
          (mkShellNoCC {
            packages =
              with pkgs;
              [
                # Necessary packages for build
                openssl
                cargo-binutils
              ]
              ++ devPkgs;
          });
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = crateMetadata.name;
          version = crateMetadata.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = {
            description = crateMetadata.description;
            homepage = crateMetadata.repository;
            license = with lib.licenses; [
              mit
              asl20
            ];
          };
        };

        # The default devShell with IDE integrations
        devShells.default = mkDevShell [ rust-dev-with-rust-analyzer ];
        # A minimal devShell without IDE integrations
        devShells.minimal = mkDevShell [ rust-dev ];
        # A minimal devShell with toolchain of MSRV
        devShells.msrv = mkDevShell [ rust-msrv ];
      }
    );
}
