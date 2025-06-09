#!/usr/bin/env bash

# Example script showing how to use rdoc-mcp in different scenarios

set -e

echo "🦀 rdoc-mcp Usage Examples"
echo "=========================="
echo

# Example 1: Quick test with nix shell
echo "1. Quick test with nix shell:"
echo "   nix shell github:cyberelf/mcp_rustdoc#rustdocs-mcp --command rdoc-mcp --help"
echo

# Example 2: Run directly
echo "2. Run directly:"
echo "   nix run github:cyberelf/mcp_rustdoc#rustdocs-mcp -- --help"
echo

# Example 3: Start SSE server
echo "3. Start SSE server:"
echo "   nix run github:cyberelf/mcp_rustdoc#rustdocs-mcp -- -s sse -a 0.0.0.0:8080"
echo

# Example 4: Start stdio server
echo "4. Start stdio server:"
echo "   nix run github:cyberelf/mcp_rustdoc#rustdocs-mcp -- -s stdio"
echo

# Example 5: Build and run locally
echo "5. Build and run locally:"
echo "   nix build github:cyberelf/mcp_rustdoc#rustdocs-mcp"
echo "   ./result/bin/rdoc-mcp --help"
echo

# Example 6: Use in development shell
echo "6. Use in development shell:"
echo "   nix develop github:cyberelf/mcp_rustdoc"
echo "   # Then inside the shell:"
echo "   rdoc-mcp --help"
echo

# Example 7: Docker usage
echo "7. Docker usage:"
echo "   nix build .#docker-image  # From the example flake"
echo "   docker load < result"
echo "   docker run -p 8080:8080 rdoc-mcp-server:latest"
echo

# Example 8: Testing the server
echo "8. Testing the SSE server (after starting):"
echo "   curl http://localhost:8080/health"
echo "   curl -X POST http://localhost:8080/mcp \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"method\": \"tools/list\", \"params\": {}}'"
echo

echo "For more advanced usage examples, see:"
echo "- examples/example-flake.nix"
echo "- DISTRIBUTION.md"
