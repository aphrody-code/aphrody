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

"""Tests for specific Hook interfaces and result types in v2."""

import unittest
from typing import Any

from google.antigravity.hooks import hooks


class BaseHookTest(unittest.IsolatedAsyncioTestCase):
    """Tests default behavior of specific Hook classes and result types."""

    def test_hook_result_defaults(self):
        """Verifies the default attributes of HookResult."""
        res = hooks.HookResult()
        self.assertTrue(res.allow)
        self.assertEqual(res.message, "")

    async def test_inspect_hook(self):
        """Verifies InspectHook can be executed."""

        class DummyInspectHook(hooks.InspectHook):
            async def run(self, context: hooks.HookContext, data: Any) -> None:
                data["called"] = True

        hook = DummyInspectHook()
        ctx = hooks.HookContext()
        data = {}
        await hook.run(ctx, data)
        self.assertTrue(data["called"])

    async def test_decide_hook(self):
        """Verifies DecideHook can be executed and returns HookResult."""

        class DummyDecideHook(hooks.DecideHook):
            async def run(
                self, context: hooks.HookContext, data: Any
            ) -> hooks.HookResult:
                return hooks.HookResult(allow=True, message="allowed")

        hook = DummyDecideHook()
        ctx = hooks.HookContext()
        res = await hook.run(ctx, None)
        self.assertTrue(res.allow)
        self.assertEqual(res.message, "allowed")

    async def test_pre_turn_hook(self):
        """Verifies PreTurnHook accepts types.Content and returns HookResult."""

        class DummyPreTurnHook(hooks.PreTurnHook):
            async def run(
                self, context: hooks.HookContext, data: Any
            ) -> hooks.HookResult:
                return hooks.HookResult(
                    allow=isinstance(data, list), message="checked"
                )

        hook = DummyPreTurnHook()
        ctx = hooks.HookContext()
        res = await hook.run(ctx, ["multimodal", "prompt"])
        self.assertTrue(res.allow)
        self.assertEqual(res.message, "checked")

    async def test_transform_hook(self):
        """Verifies TransformHook can be executed and modifies data."""

        class DummyTransformHook(hooks.TransformHook):
            async def run(self, context: hooks.HookContext, data: Any) -> Any:
                return data + "_modified"

        hook = DummyTransformHook()
        ctx = hooks.HookContext()
        res = await hook.run(ctx, "original")
        self.assertEqual(res, "original_modified")

    async def test_on_compaction_hook(self):
        """Verifies OnCompactionHook can be instantiated and executed."""

        class DummyCompactionHook(hooks.OnCompactionHook):
            async def run(self, context: hooks.HookContext, data: Any) -> None:
                data["compaction_observed"] = True

        hook = DummyCompactionHook()
        ctx = hooks.HookContext()
        data = {}
        await hook.run(ctx, data)
        self.assertTrue(data["compaction_observed"])


if __name__ == "__main__":
    unittest.main()
