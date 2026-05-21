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

"""Tests for TriggerRunner lifecycle management."""

import asyncio
import logging
import unittest
from unittest import mock

from google.antigravity.connections import connection
from google.antigravity.triggers import trigger_runner, triggers


class TriggerRunnerTest(unittest.IsolatedAsyncioTestCase):
    def _make_conn(self):
        conn = mock.AsyncMock(spec=connection.Connection)
        conn.send = mock.AsyncMock()
        conn.wait_for_idle = mock.AsyncMock()
        return conn

    async def test_start_runs_triggers(self):
        started = asyncio.Event()

        async def my_trigger(ctx: triggers.TriggerContext) -> None:
            started.set()
            # Keep alive until cancelled.
            await asyncio.Event().wait()

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[my_trigger], connection=conn
        )

        await runner.start()
        # Wait for the trigger to signal it started.
        await asyncio.wait_for(started.wait(), timeout=1.0)
        self.assertTrue(runner.is_running)

        await runner.stop()
        self.assertFalse(runner.is_running)

    async def test_stop_cancels_all_triggers(self):
        cancelled_triggers = []

        async def my_trigger(ctx: triggers.TriggerContext) -> None:
            try:
                await asyncio.Event().wait()
            except asyncio.CancelledError:
                cancelled_triggers.append(True)
                raise

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[my_trigger, my_trigger], connection=conn
        )

        await runner.start()
        await asyncio.sleep(0)  # Let trigger tasks begin.
        await runner.stop()

        self.assertEqual(len(cancelled_triggers), 2)

    async def test_exception_in_trigger_does_not_crash_others(self):
        healthy_ran = asyncio.Event()

        async def bad_trigger(ctx: triggers.TriggerContext) -> None:
            raise ValueError("boom")

        async def good_trigger(ctx: triggers.TriggerContext) -> None:
            healthy_ran.set()
            await asyncio.Event().wait()

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[bad_trigger, good_trigger], connection=conn
        )

        with self.assertLogs(level=logging.ERROR) as logs:
            await runner.start()
            await asyncio.wait_for(healthy_ran.wait(), timeout=1.0)

        # Bad trigger logged its error.
        self.assertTrue(
            any("boom" in log for log in logs.output),
            f"Expected 'boom' in logs, got: {logs.output}",
        )

        # Good trigger is still running.
        self.assertTrue(runner.is_running)
        await runner.stop()

    async def test_start_twice_raises(self):
        async def my_trigger(ctx: triggers.TriggerContext) -> None:
            await asyncio.Event().wait()

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[my_trigger], connection=conn
        )

        await runner.start()
        with self.assertRaises(RuntimeError):
            await runner.start()
        await runner.stop()

    async def test_stop_when_not_started_is_noop(self):
        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(triggers=[], connection=conn)

        # Should not raise.
        await runner.stop()

    async def test_empty_triggers_list(self):
        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(triggers=[], connection=conn)

        await runner.start()
        self.assertFalse(runner.is_running)
        await runner.stop()

    async def test_trigger_receives_context_with_connection(self):
        received_ctx = []

        async def capture_trigger(ctx: triggers.TriggerContext) -> None:
            received_ctx.append(ctx)

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[capture_trigger], connection=conn
        )

        await runner.start()
        # Give the trigger time to execute.
        await asyncio.sleep(0.01)
        await runner.stop()

        self.assertEqual(len(received_ctx), 1)

    async def test_restart_after_stop(self):
        call_count = 0

        async def counting_trigger(ctx: triggers.TriggerContext) -> None:
            nonlocal call_count
            call_count += 1

        conn = self._make_conn()
        runner = trigger_runner.TriggerRunner(
            triggers=[counting_trigger], connection=conn
        )

        await runner.start()
        await asyncio.sleep(0.01)
        await runner.stop()

        first_count = call_count

        # Should be able to start again after stop on the same runner.
        await runner.start()
        await asyncio.sleep(0.01)
        await runner.stop()

        self.assertGreater(call_count, first_count)

    async def test_context_manager_starts_and_stops(self):
        started = asyncio.Event()

        async def my_trigger(ctx: triggers.TriggerContext) -> None:
            started.set()
            await asyncio.Event().wait()

        conn = self._make_conn()
        async with trigger_runner.TriggerRunner(
            triggers=[my_trigger], connection=conn
        ) as runner:
            await asyncio.wait_for(started.wait(), timeout=1.0)
            self.assertTrue(runner.is_running)

        # After exiting context, triggers should be stopped.
        self.assertFalse(runner.is_running)


if __name__ == "__main__":
    unittest.main()
