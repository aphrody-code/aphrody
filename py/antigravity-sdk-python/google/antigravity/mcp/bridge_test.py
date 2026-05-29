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

"""Unit tests for bridge.py."""

import asyncio
import unittest
from unittest import mock

from absl.testing import parameterized
from google.antigravity import types as sdk_types
from google.antigravity.mcp import bridge as bridge_module
from google.antigravity.mcp.bridge import (
    McpBridge,
    _component_name_hook,
    _current_server_cfg_var,
    get_mcp_tools,
)
from mcp import types
from mcp.client.session_group import ClientSessionGroup


class TestBridge(unittest.TestCase):
    def test_get_mcp_tools(self):
        mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
        mock_tool = types.Tool(
            name="test_tool",
            description="A test tool",
            inputSchema={"type": "object"},
        )
        mock_session_group.tools = {"test_tool": mock_tool}
        mock_session_group.call_tool = mock.AsyncMock(
            return_value="tool_result"
        )

        async def run_test():
            tools = await get_mcp_tools(mock_session_group)

            self.assertEqual(len(tools), 1)
            wrapper_fn = tools[0]
            self.assertEqual(wrapper_fn.__name__, "test_tool")
            self.assertEqual(wrapper_fn.__doc__, "A test tool")

            result = await wrapper_fn(arg1="val1")
            self.assertEqual(result, "tool_result")
            mock_session_group.call_tool.assert_called_once_with(
                "test_tool", {"arg1": "val1"}
            )

        asyncio.run(run_test())


class TestMcpBridge(unittest.TestCase):
    def test_connect_stdio(self):
        """Verifies that connect_stdio correctly configures stdio transport."""
        bridge = McpBridge()

        patch_target = "google.antigravity.mcp.bridge.ClientSessionGroup"
        with mock.patch(patch_target) as mock_group_cls:
            mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
            mock_group_cls.return_value = mock_session_group
            mock_session_group.__aenter__ = mock.AsyncMock(
                return_value=mock_session_group
            )
            mock_session_group.connect_to_server = mock.AsyncMock()
            mock_session_group.tools = {}

            async def run_test():
                await bridge.connect_stdio(
                    "pirate_command", ["--transport=stdio"]
                )
                mock_session_group.connect_to_server.assert_called_once()

            asyncio.run(run_test())

    def test_connect_sse(self):
        """Verifies that connect_sse correctly configures SSE transport parameters."""
        bridge = McpBridge()

        patch_target = "google.antigravity.mcp.bridge.ClientSessionGroup"
        with mock.patch(patch_target) as mock_group_cls:
            mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
            mock_group_cls.return_value = mock_session_group
            mock_session_group.__aenter__ = mock.AsyncMock(
                return_value=mock_session_group
            )
            mock_session_group.connect_to_server = mock.AsyncMock()
            mock_session_group.tools = {}

            async def run_test():
                await bridge.connect_sse("http://localhost:8080/sse")
                mock_session_group.connect_to_server.assert_called_once()

            asyncio.run(run_test())

    def test_connect_streamable_http(self):
        """Verifies that connect_streamable_http correctly configures HTTP transport parameters."""
        bridge = McpBridge()

        patch_target = "google.antigravity.mcp.bridge.ClientSessionGroup"
        with mock.patch(patch_target) as mock_group_cls:
            mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
            mock_group_cls.return_value = mock_session_group
            mock_session_group.__aenter__ = mock.AsyncMock(
                return_value=mock_session_group
            )
            mock_session_group.connect_to_server = mock.AsyncMock()
            mock_session_group.tools = {}

            async def run_test():
                await bridge.connect_streamable_http(
                    "http://localhost:8080/mcp"
                )
                mock_session_group.connect_to_server.assert_called_once()

                args, _ = mock_session_group.connect_to_server.call_args
                params = args[0]
                self.assertEqual(params.url, "http://localhost:8080/mcp")
                self.assertEqual(params.terminate_on_close, True)

                # Test with terminate_on_close=False
                mock_session_group.connect_to_server.reset_mock()
                await bridge.connect_streamable_http(
                    "http://localhost:8080/mcp", terminate_on_close=False
                )
                mock_session_group.connect_to_server.assert_called_once()
                args, _ = mock_session_group.connect_to_server.call_args
                params = args[0]
                self.assertEqual(params.terminate_on_close, False)

            asyncio.run(run_test())

    def test_connect(self):
        """Verifies that connect correctly dispatches to specific methods."""
        bridge = McpBridge()

        bridge.connect_stdio = mock.AsyncMock()
        bridge.connect_sse = mock.AsyncMock()
        bridge.connect_streamable_http = mock.AsyncMock()

        async def run_test():
            # Test stdio
            stdio_cfg = mock.MagicMock()
            stdio_cfg.type = "stdio"
            stdio_cfg.name = "stdio_math"
            stdio_cfg.command = "cmd"
            stdio_cfg.args = ["arg"]
            await bridge.connect(stdio_cfg)
            bridge.connect_stdio.assert_called_once_with(
                "cmd", ["arg"], server_cfg=stdio_cfg
            )

            # Test sse
            sse_cfg = mock.MagicMock()
            sse_cfg.type = "sse"
            sse_cfg.name = "sse_math"
            sse_cfg.url = "url"
            sse_cfg.headers = {"h": "v"}
            await bridge.connect(sse_cfg)
            bridge.connect_sse.assert_called_once_with(
                "url", {"h": "v"}, server_cfg=sse_cfg
            )

            # Test http
            http_cfg = mock.MagicMock()
            http_cfg.type = "http"
            http_cfg.name = "http_math"
            http_cfg.url = "url2"
            http_cfg.headers = None
            http_cfg.timeout = 10.0
            http_cfg.sse_read_timeout = 20.0
            http_cfg.terminate_on_close = False
            await bridge.connect(http_cfg)
            bridge.connect_streamable_http.assert_called_once_with(
                url="url2",
                headers=None,
                timeout=10.0,
                sse_read_timeout=20.0,
                terminate_on_close=False,
                server_cfg=http_cfg,
            )

        asyncio.run(run_test())

    def test_stop(self):
        """Verifies that McpBridge stopped safely exiting ClientSessionGroup contexts."""
        bridge = McpBridge()

        patch_target = "google.antigravity.mcp.bridge.ClientSessionGroup"
        with mock.patch(patch_target) as mock_group_cls:
            mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
            mock_group_cls.return_value = mock_session_group
            mock_session_group.__aenter__ = mock.AsyncMock(
                return_value=mock_session_group
            )
            mock_session_group.__aexit__ = mock.AsyncMock()
            mock_session_group.connect_to_server = mock.AsyncMock()
            mock_session_group.tools = {}

            async def run_test():
                await bridge.connect_stdio(
                    "pirate_command", ["--transport=stdio"]
                )
                await bridge.stop()
                mock_session_group.__aexit__.assert_called_once()

            asyncio.run(run_test())


class TestMcpBridgeFiltering(unittest.TestCase):
    def _setup_mock_session_group(self, mock_group_cls, tools_to_add):
        """Helper to set up ClientSessionGroup mock with tools added during connect."""
        mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
        mock_group_cls.return_value = mock_session_group
        mock_session_group.__aenter__ = mock.AsyncMock(
            return_value=mock_session_group
        )

        tools_dict = {}
        mock_session_group.tools = tools_dict

        async def mock_connect(params):
            tools_dict.update(tools_to_add)

        mock_session_group.connect_to_server = mock.AsyncMock(
            side_effect=mock_connect
        )
        return mock_session_group

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_without_filtering(self, mock_group_cls):
        """Verifies that all tools are kept when no filtering is configured."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_my_server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
                "mcp_my_server_tool2": types.Tool(
                    name="tool2",
                    description="tool2 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        server_cfg = sdk_types.McpStdioServer(name="my_server", command="cmd")

        async def run_test():
            await bridge.connect(server_cfg)
            self.assertEqual(len(bridge.tools), 2)
            tool_names = {t.__name__ for t in bridge.tools}
            self.assertEqual(
                tool_names, {"mcp_my_server_tool1", "mcp_my_server_tool2"}
            )

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_with_disabled_tools(self, mock_group_cls):
        """Verifies that disabled tools are filtered out."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_my_server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
                "mcp_my_server_tool2": types.Tool(
                    name="tool2",
                    description="tool2 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        server_cfg = sdk_types.McpStdioServer(
            name="my_server", command="cmd", disabled_tools=["tool2"]
        )

        async def run_test():
            await bridge.connect(server_cfg)
            self.assertEqual(len(bridge.tools), 1)
            self.assertEqual(bridge.tools[0].__name__, "mcp_my_server_tool1")

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_with_enabled_tools(self, mock_group_cls):
        """Verifies that only enabled tools are kept."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_my_server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
                "mcp_my_server_tool2": types.Tool(
                    name="tool2",
                    description="tool2 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        server_cfg = sdk_types.McpStdioServer(
            name="my_server", command="cmd", enabled_tools=["tool1"]
        )

        async def run_test():
            await bridge.connect(server_cfg)
            self.assertEqual(len(bridge.tools), 1)
            self.assertEqual(bridge.tools[0].__name__, "mcp_my_server_tool1")

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_with_invalid_enabled_tools_raises(self, mock_group_cls):
        """Verifies that ValueError is raised if configured enabled_tools do not exist."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_my_server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        server_cfg = sdk_types.McpStdioServer(
            name="my_server", command="cmd", enabled_tools=["non_existent_tool"]
        )

        async def run_test():
            with self.assertRaises(ValueError) as ctx:
                await bridge.connect(server_cfg)
            self.assertIn(
                "Configured enabled_tools do not exist", str(ctx.exception)
            )

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_with_invalid_disabled_tools_raises(self, mock_group_cls):
        """Verifies that ValueError is raised if configured disabled_tools do not exist."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_my_server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        server_cfg = sdk_types.McpStdioServer(
            name="my_server",
            command="cmd",
            disabled_tools=["non_existent_tool"],
        )

        async def run_test():
            with self.assertRaises(ValueError) as ctx:
                await bridge.connect(server_cfg)
            self.assertIn(
                "Configured disabled_tools do not exist", str(ctx.exception)
            )

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_multiple_servers_filtering(self, mock_group_cls):
        """Verifies filtering works independently for multiple servers."""
        bridge = McpBridge()

        # For multi-server, we need a custom side effect to simulate successive connects
        mock_session_group = mock.MagicMock(spec=ClientSessionGroup)
        mock_group_cls.return_value = mock_session_group
        mock_session_group.__aenter__ = mock.AsyncMock(
            return_value=mock_session_group
        )

        tools_dict = {}
        mock_session_group.tools = tools_dict

        # First connection side effect
        async def mock_connect1(params):
            tools_dict.update(
                {
                    "mcp_server1_tool1": types.Tool(
                        name="tool1",
                        description="tool1 desc",
                        inputSchema={"type": "object"},
                    ),
                    "mcp_server1_tool2": types.Tool(
                        name="tool2",
                        description="tool2 desc",
                        inputSchema={"type": "object"},
                    ),
                }
            )

        mock_session_group.connect_to_server = mock.AsyncMock(
            side_effect=mock_connect1
        )

        server1_cfg = sdk_types.McpStdioServer(
            name="server1", command="cmd1", disabled_tools=["tool2"]
        )

        # Second connection side effect (appends to existing tools)
        async def mock_connect2(params):
            tools_dict.update(
                {
                    "mcp_server2_tool1": types.Tool(
                        name="tool1",
                        description="tool1 desc",
                        inputSchema={"type": "object"},
                    ),
                    "mcp_server2_tool2": types.Tool(
                        name="tool2",
                        description="tool2 desc",
                        inputSchema={"type": "object"},
                    ),
                }
            )

        async def run_test():
            # Connect first server
            await bridge.connect(server1_cfg)
            self.assertEqual(len(bridge.tools), 1)
            self.assertEqual(bridge.tools[0].__name__, "mcp_server1_tool1")

            # Re-mock side effect for second connection
            mock_session_group.connect_to_server.side_effect = mock_connect2
            server2_cfg = sdk_types.McpStdioServer(
                name="server2", command="cmd2", enabled_tools=["tool2"]
            )

            await bridge.connect(server2_cfg)

            # Total tools should be server1_tool1 (allowed) + server2_tool2 (allowed)
            self.assertEqual(len(bridge.tools), 2)
            tool_names = {t.__name__ for t in bridge.tools}
            self.assertEqual(
                tool_names, {"mcp_server1_tool1", "mcp_server2_tool2"}
            )

        asyncio.run(run_test())

    @mock.patch("google.antigravity.mcp.bridge.ClientSessionGroup")
    def test_connect_case_preserving_filtering(self, mock_group_cls):
        """Verifies that tool matching and prefix stripping preserve case."""
        bridge = McpBridge()
        self._setup_mock_session_group(
            mock_group_cls,
            {
                "mcp_My-Server_tool1": types.Tool(
                    name="tool1",
                    description="tool1 desc",
                    inputSchema={"type": "object"},
                ),
                "mcp_My-Server_tool2": types.Tool(
                    name="tool2",
                    description="tool2 desc",
                    inputSchema={"type": "object"},
                ),
            },
        )

        # Server name has uppercase letters
        server_cfg = sdk_types.McpStdioServer(
            name="My-Server", command="cmd", disabled_tools=["tool2"]
        )

        async def run_test():
            await bridge.connect(server_cfg)
            self.assertEqual(len(bridge.tools), 1)
            # Prefixed name must preserve case exactly
            self.assertEqual(bridge.tools[0].__name__, "mcp_My-Server_tool1")

        asyncio.run(run_test())


class TestMcpToolPrefixing(parameterized.TestCase):
    """Test suite for namespaced tool prefixing behaviors."""

    @parameterized.named_parameters(
        dict(
            testcase_name="with_config_name",
            config_name="Math-Server",
            server_info_name=None,
            tool_name="add",
            expected="mcp_Math-Server_add",
        ),
        dict(
            testcase_name="with_server_info_fallback",
            config_name=None,
            server_info_name="Math Server (v3!)",
            tool_name="add",
            expected="mcp_Math_Server_v3_add",
        ),
        dict(
            testcase_name="with_no_name_anywhere",
            config_name=None,
            server_info_name=None,
            tool_name="add",
            expected="mcp_add",
        ),
    )
    def test_component_name_hook(
        self, config_name, server_info_name, tool_name, expected
    ):
        """Verifies prefix and sanitization logic under different connection contexts."""
        server_cfg = None
        if config_name:
            server_cfg = mock.create_autospec(
                sdk_types.McpStdioServer, instance=True
            )
            server_cfg.name = config_name

        server_info = None
        if server_info_name:
            server_info = mock.create_autospec(
                types.Implementation, instance=True
            )
            server_info.name = server_info_name

        token = None
        if server_cfg:
            token = _current_server_cfg_var.set(server_cfg)

        try:
            resolved = _component_name_hook(tool_name, server_info)
            self.assertEqual(resolved, expected)
        finally:
            if token:
                _current_server_cfg_var.reset(token)

    def test_connect_registers_component_name_hook(self):
        """Verifies the bridge correctly registers the renaming hook during connection."""
        bridge_inst = McpBridge()
        with mock.patch.object(
            bridge_module, "ClientSessionGroup"
        ) as mock_group_cls:
            mock_session_group = mock.create_autospec(
                ClientSessionGroup, instance=True
            )
            mock_group_cls.return_value = mock_session_group
            mock_session_group.__aenter__ = mock.AsyncMock(
                return_value=mock_session_group
            )
            mock_session_group.connect_to_server = mock.AsyncMock()
            mock_session_group.tools = {}

            async def run_test():
                await bridge_inst.connect_stdio("cmd", ["arg"])
                mock_group_cls.assert_called_once_with(
                    component_name_hook=_component_name_hook
                )

            asyncio.run(run_test())


if __name__ == "__main__":
    unittest.main()
