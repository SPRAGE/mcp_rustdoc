{
  description = "MCP implementation for querying Rust API documentation from docs.rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        commonAttrs = {
          pname = "rdoc-mcp";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          meta = with pkgs.lib; {
            description = "MCP implementation for querying Rust API documentation from docs.rs";
            homepage = "https://github.com/cyberelf/mcp_rustdoc";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.unix;
          };
        };

        rustdocs-mcp = rustPlatform.buildRustPackage commonAttrs;

      in
      {
        packages = {
          default = rustdocs-mcp;
          rustdocs-mcp = rustdocs-mcp;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = rustdocs-mcp;
            name = "rdoc-mcp";
          };
          rustdocs-mcp = flake-utils.lib.mkApp {
            drv = rustdocs-mcp;
            name = "rdoc-mcp";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            
            # Cargo tools
            cargo-watch
            cargo-edit
            cargo-audit
            cargo-deny
            cargo-expand
            cargo-udeps
            
            # Development tools
            rust-analyzer
            clippy
            rustfmt
            
            # Additional utilities
            git
            fd
            ripgrep
          ];

          shellHook = ''
            echo "🦀 Rust development environment loaded!"
            echo "Available tools:"
            echo "  - cargo watch  : cargo-watch"
            echo "  - cargo edit   : cargo-edit"
            echo "  - cargo audit  : cargo-audit"
            echo "  - cargo deny   : cargo-deny"
            echo "  - cargo expand : cargo-expand"
            echo "  - cargo udeps  : cargo-udeps"
            echo "  - rust-analyzer: Language server"
            echo ""
            echo "Project: rustdocs-mcp"
            echo "Run 'cargo run' to start the MCP server"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          RUST_LOG = "debug";
        };

        # Export the package for use in other flakes
        lib = {
          inherit rustdocs-mcp;
        };
      });
}
