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
      in {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              # Rust toolchain
              rustc
              cargo
              clippy
              rustfmt

              # Build dependencies
              openssl
              fuse3
              pkg-config

              # Database tools (for integration tests)
              postgresql

              # Docker (for running PostgreSQL in tests)
              docker
              docker-compose

              # Network and process utilities (for test scripts)
              iproute2  # provides ss command
              psmisc    # provides fuser command
            ];

            shellHook = ''
              echo "BlogGen development environment loaded"
              echo ""
              echo "Available commands:"
              echo "  cargo test          - Run unit tests"
              echo "  ./run-tests.sh      - Run all tests (unit + integration)"
              echo "  ./run-tests.sh --unit        - Run only unit tests"
              echo "  ./run-tests.sh --integration - Run only integration tests"
              echo "  ./run-tests.sh --verbose     - Run with verbose output"
              echo ""
              echo "Note: Integration tests require Docker daemon to be running"
              echo "      Run 'systemctl start docker' or equivalent for your system"
              echo ""
            '';

            # Environment variables
            RUST_BACKTRACE = 1;

            # Ensure Docker socket is accessible
            DOCKER_HOST = "unix:///var/run/docker.sock";
          };
        };
      });
}
