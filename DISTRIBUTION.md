# Distributing and Using rdoc-mcp in Other Flakes

This document explains how to distribute your `rdoc-mcp` package and use it in other Nix flakes.

## Building the Package

Build the package locally:
```bash
nix build .#rustdocs-mcp
```

Or run it directly:
```bash
nix run .#rustdocs-mcp -- --help
```

## Using in Other Flakes

There are several ways to include `rdoc-mcp` in other flakes:

### Method 1: Direct Flake Input

Add to your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rdoc-mcp = {
      url = "github:cyberelf/mcp_rustdoc";  # Update with your repo URL
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rdoc-mcp }:
    let
      system = "x86_64-linux";  # or your target system
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.${system}.default = rdoc-mcp.packages.${system}.rustdocs-mcp;
      
      # Or use in a development shell
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rdoc-mcp.packages.${system}.rustdocs-mcp
        ];
      };
    };
}
```

### Method 2: Using the Overlay

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  };

  outputs = { self, nixpkgs, rdoc-mcp }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rdoc-mcp.overlays.${system}.default ];
      };
    in
    {
      packages.${system}.default = pkgs.rdoc-mcp;
    };
}
```

### Method 3: Using the Library Export

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  };

  outputs = { self, nixpkgs, rdoc-mcp }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      rdocMcp = rdoc-mcp.lib.${system}.rustdocs-mcp;
    in
    {
      packages.${system}.my-package = pkgs.stdenv.mkDerivation {
        name = "my-package";
        buildInputs = [ rdocMcp ];
        # ... rest of your derivation
      };
    };
}
```

## Building a Docker Image

You can also create a Docker image with your MCP server:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  };

  outputs = { self, nixpkgs, rdoc-mcp }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.${system}.docker-image = pkgs.dockerTools.buildImage {
        name = "rdoc-mcp";
        tag = "latest";
        contents = [ rdoc-mcp.packages.${system}.rustdocs-mcp ];
        config = {
          Cmd = [ "${rdoc-mcp.packages.${system}.rustdocs-mcp}/bin/rdoc-mcp" ];
          ExposedPorts = {
            "8080/tcp" = {};
          };
        };
      };
    };
}
```

Build the Docker image:
```bash
nix build .#docker-image
docker load < result
docker run -p 8080:8080 rdoc-mcp:latest
```

## NixOS System Integration

Include in your NixOS configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  };

  outputs = { self, nixpkgs, rdoc-mcp }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [
            rdoc-mcp.packages.x86_64-linux.rustdocs-mcp
          ];

          # Optional: Create a systemd service
          systemd.services.rdoc-mcp = {
            description = "Rust Documentation MCP Server";
            wantedBy = [ "multi-user.target" ];
            serviceConfig = {
              ExecStart = "${rdoc-mcp.packages.x86_64-linux.rustdocs-mcp}/bin/rdoc-mcp -s sse -a 0.0.0.0:8080";
              Restart = "always";
              User = "nobody";
              Group = "nogroup";
            };
          };
        })
      ];
    };
  };
}
```

## Home Manager Integration

For user-level installation:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager.url = "github:nix-community/home-manager";
    rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  };

  outputs = { self, nixpkgs, home-manager, rdoc-mcp }: {
    homeConfigurations.myuser = home-manager.lib.homeManagerConfiguration {
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
      modules = [
        {
          home.packages = [
            rdoc-mcp.packages.x86_64-linux.rustdocs-mcp
          ];
        }
      ];
    };
  };
}
```

## Using with Nix Shell

For quick usage without a flake:

```bash
nix shell github:cyberelf/mcp_rustdoc#rustdocs-mcp
rdoc-mcp --help
```

## Publication to GitHub

1. **Create a repository** on GitHub (if not already done)
2. **Push your flake** with the properly configured `flake.nix`
3. **Tag a release** for stable versions:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. **Reference by tag** in other flakes:
   ```nix
   rdoc-mcp.url = "github:cyberelf/mcp_rustdoc/v0.1.0";
   ```

## Publishing to FlakeHub

[FlakeHub](https://flakehub.com) provides a registry for Nix flakes:

1. **Sign up** at flakehub.com
2. **Connect your GitHub repository**
3. **Configure publishing** in your repository
4. **Reference** via FlakeHub:
   ```nix
   rdoc-mcp.url = "https://flakehub.com/f/cyberelf/mcp_rustdoc/0.1.0.tar.gz";
   ```

## Binary Cache

For faster builds, consider setting up a binary cache:

1. **Using Cachix**:
   ```bash
   nix-env -iA cachix -f https://cachix.org/api/v1/install
   cachix generate-keypair <your-cache-name>
   cachix push <your-cache-name> $(nix-build)
   ```

2. **Using in other flakes**:
   ```nix
   nixConfig = {
     extra-substituters = [ "https://<your-cache-name>.cachix.org" ];
     extra-trusted-public-keys = [ "<your-cache-name>.cachix.org-1:..." ];
   };
   ```

## Cross-Platform Support

The flake supports multiple platforms. To build for different architectures:

```bash
# Build for macOS ARM64
nix build .#rustdocs-mcp --system aarch64-darwin

# Build for macOS x86_64
nix build .#rustdocs-mcp --system x86_64-darwin

# Build for Linux ARM64
nix build .#rustdocs-mcp --system aarch64-linux
```

## Testing Your Distribution

Create a minimal test flake to verify your package works:

```nix
{
  inputs.rdoc-mcp.url = "github:cyberelf/mcp_rustdoc";
  
  outputs = { self, rdoc-mcp }:
    let system = "x86_64-linux"; in
    {
      packages.${system}.test = rdoc-mcp.packages.${system}.rustdocs-mcp;
      
      checks.${system}.test-run = derivation {
        inherit system;
        name = "test-rdoc-mcp";
        builder = "${rdoc-mcp.packages.${system}.rustdocs-mcp}/bin/rdoc-mcp";
        args = [ "--help" ];
      };
    };
}
```

Run the test:
```bash
nix build .#test
nix flake check
```

This comprehensive setup allows you to distribute your `rdoc-mcp` package easily and makes it simple for others to include it in their Nix flakes.
