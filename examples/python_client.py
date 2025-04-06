import asyncio
from typing import Optional
from contextlib import AsyncExitStack

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


class MCPClient:
    def __init__(self):
        # Initialize session and client objects
        self.session: Optional[ClientSession] = None
        self.exit_stack = AsyncExitStack()
        
    async def connect_to_server(self, server_path: str):
        """Connect to an MCP server

        Args:
            server_path: Path to the server script (.py or .js)
        """

        server_params = StdioServerParameters(
            command=server_path,
            args=["--server-type", "stdio"],
            env=None
        )

        stdio_transport = await self.exit_stack.enter_async_context(stdio_client(server_params))
        self.stdio, self.write = stdio_transport
        self.session = await self.exit_stack.enter_async_context(ClientSession(self.stdio, self.write))

        await self.session.initialize()

        # List available tools
        response = await self.session.list_tools()
        tools = response.tools
        print("\nConnected to server with tools:", [tool.name for tool in tools])

        # list all resources
        response = await self.session.list_resources()
        resources = response.resources
        print("\nResources:", [resource.name for resource in resources])

        # list all prompts
        response = await self.session.list_prompts()
        prompts = response.prompts
        print("\nPrompts:", [prompt.name for prompt in prompts])

    async def disconnect_from_server(self):
        """Disconnect from the MCP server"""
        await self.exit_stack.aclose()

    async def call_tool(self, tool_name: str, tool_args: dict):
        """Call a tool with the given name and arguments"""
        response = await self.session.call_tool(tool_name, tool_args)
        return response.content[0].text


async def main():
    client = MCPClient()
    try:
        await client.connect_to_server("target/release/rust-mcp")
        result = await client.call_tool("fetch_document", {"crate_name": "rand", "version": "0.9.0", "path": "rand"})
        print(result)
    finally:
        print("Disconnecting...") # Optional: for clarity
        await client.disconnect_from_server()


if __name__ == "__main__":
    client = MCPClient()
    asyncio.run(main())
