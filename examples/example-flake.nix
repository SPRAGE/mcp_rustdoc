# Example flake demonstrating how to use rdoc-mcp
{
  description = "Example project using rdoc-mcp";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    
    # Include your rdoc-mcp flake
    rdoc-mcp = {
      url = "github:cyberelf/mcp_rustdoc";  # Replace with your actual repo URL
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rdoc-mcp }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        # Make rdoc-mcp available as a package
        packages = {
          default = rdoc-mcp.packages.${system}.rustdocs-mcp;
          rdoc-mcp = rdoc-mcp.packages.${system}.rustdocs-mcp;
          
          # Example: Create a Docker image with rdoc-mcp
          docker-image = pkgs.dockerTools.buildImage {
            name = "rdoc-mcp-server";
            tag = "latest";
            contents = [ rdoc-mcp.packages.${system}.rustdocs-mcp ];
            config = {
              Cmd = [ "${rdoc-mcp.packages.${system}.rustdocs-mcp}/bin/rdoc-mcp" "-s" "sse" "-a" "0.0.0.0:8080" ];
              ExposedPorts = {
                "8080/tcp" = {};
              };
            };
          };
        };

        # Apps for running rdoc-mcp directly
        apps = {
          default = rdoc-mcp.apps.${system}.default;
          rdoc-mcp = rdoc-mcp.apps.${system}.rustdocs-mcp;
          
          # Custom app that starts the server with specific settings
          rdoc-server = {
            type = "app";
            program = "${rdoc-mcp.packages.${system}.rustdocs-mcp}/bin/rdoc-mcp";
            args = ["-s" "sse" "-a" "0.0.0.0:8080"];
          };
        };

        # Development shell that includes rdoc-mcp
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Include the rdoc-mcp binary
            rdoc-mcp.packages.${system}.rustdocs-mcp
            
            # Other development tools
            curl
            jq
            httpie
          ];

          shellHook = ''
            echo "🦀 Development environment with rdoc-mcp loaded!"
            echo ""
            echo "Available commands:"
            echo "  rdoc-mcp --help    : Show rdoc-mcp help"
            echo "  rdoc-mcp -s sse    : Start SSE server (default)"
            echo "  rdoc-mcp -s stdio  : Start stdio server"
            echo ""
            echo "Test the server:"
            echo "  curl http://localhost:8080  (after starting SSE server)"
            echo ""
          '';
        };

        # NixOS module for system-wide installation
        nixosModules.rdoc-mcp = { config, lib, pkgs, ... }: {
          options.services.rdoc-mcp = {
            enable = lib.mkEnableOption "rdoc-mcp server";
            
            address = lib.mkOption {
              type = lib.types.str;
              default = "127.0.0.1:8080";
              description = "Address and port for the rdoc-mcp server";
            };
            
            openFirewall = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Whether to open the firewall for rdoc-mcp";
            };
          };

          config = lib.mkIf config.services.rdoc-mcp.enable {
            systemd.services.rdoc-mcp = {
              description = "Rust Documentation MCP Server";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];
              
              serviceConfig = {
                ExecStart = "${rdoc-mcp.packages.${system}.rustdocs-mcp}/bin/rdoc-mcp -s sse -a ${config.services.rdoc-mcp.address}";
                Restart = "always";
                RestartSec = "10";
                User = "nobody";
                Group = "nogroup";
                
                # Security hardening
                NoNewPrivileges = true;
                PrivateTmp = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                ProtectKernelTunables = true;
                ProtectKernelModules = true;
                ProtectControlGroups = true;
              };
            };

            networking.firewall.allowedTCPPorts = lib.mkIf config.services.rdoc-mcp.openFirewall [
              (lib.toInt (lib.last (lib.splitString ":" config.services.rdoc-mcp.address)))
            ];
          };
        };
      });
}
