{
  description = "A simple blogging framework";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: 
    flake-utils.lib.eachDefaultSystem(system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        
        commonTools = with pkgs; [
          git
        ];

        rustPkgs = with pkgs; [
          rustc
          cargo
          clippy
          rustfmt
        ];
      in {
        packages = {
          server = pkgs.rustPlatform.buildRustPackage {
            pname = "bloggen-server";
            version = "0.2.0";
            src = ./server;
            cargoLock = {
              lockFile = ./server/Cargo.lock;
            };
          };

          go_client = pkgs.buildGoModule {
            pname = "bloggen-cli";
            version = "0.1.0";
            src = ./cli;
            vendorHash = null;
          };
        };

        devShells = {
          default = pkgs.mkShell {
            buildInputs = commonTools ++ rustPkgs ++ [
              pkgs.go
              pkgs.nodejs_20
            ];
          };

          rust = pkgs.mkShell {
            buildInputs = commonTools ++ rustPkgs ++ [
              pkgs.openssl
            ];

            # more env vars here
            RUST_BACKTRACE = 1;
          };

          go = pkgs.mkShell {
            buildInputs = commonTools ++ [
              pkgs.go
            ];
          };
        };
      });
}
