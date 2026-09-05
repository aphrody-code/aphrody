# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""MCP Integration example for Google Antigravity SDK.

This example demonstrates how to connect an agent to external MCP servers
using stdio, SSE, and Streamable HTTP transports.

To run:
  python mcp_tools.py
"""

import asyncio
import os
import sys

from google.antigravity import Agent, LocalAgentConfig, types

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from resources import mcp_server


async def mcp_stdio() -> None:
    """Showcases the Stdio transport."""
    print("\n  --- Showcasing Stdio Transport ---")
    mcp_server_path = os.path.join(
        os.path.dirname(__file__), "..", "resources", "mcp_server.py"
    )
    stdio_server = types.McpStdioServer(
        command="python3",
        args=[mcp_server_path, "--transport=stdio"],
    )

    config = LocalAgentConfig(mcp_servers=[stdio_server])

    async with Agent(config) as my_agent:
        prompt = "Use the pirate_multiply tool to multiply 5 and 7."
        print(f"  User: {prompt}")
        response = await my_agent.chat(prompt)
        print(f"  Agent: {await response.text()}")


async def mcp_sse() -> None:
    """Showcases the SSE transport."""
    print("\n  --- Showcasing SSE Transport ---")
    async with mcp_server.run("sse") as port:
        config = LocalAgentConfig(
            mcp_servers=[types.McpSseServer(url=f"http://localhost:{port}/sse")]
        )

        async with Agent(config) as my_agent:
            prompt = "Use the pirate_multiply tool to multiply 5 and 7."
            print(f"  User: {prompt}")
            response = await my_agent.chat(prompt)
            print(f"  Agent: {await response.text()}")


async def mcp_http() -> None:
    """Showcases the Streamable HTTP transport."""
    print("\n  --- Showcasing Streamable HTTP Transport ---")
    async with mcp_server.run("streamable-http") as port:
        config = LocalAgentConfig(
            mcp_servers=[
                types.McpStreamableHttpServer(
                    url=f"http://localhost:{port}/mcp"
                )
            ]
        )

        async with Agent(config) as my_agent:
            prompt = "Use the pirate_multiply tool to multiply 5 and 7."
            print(f"  User: {prompt}")
            response = await my_agent.chat(prompt)
            print(f"  Agent: {await response.text()}")


async def main() -> None:
    await mcp_stdio()
    await mcp_sse()
    await mcp_http()


if __name__ == "__main__":
    asyncio.run(main())
