# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Subprocess execution timeout and runaway loop monitoring."""

import logging
import subprocess
import time
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from concurrent.futures import TimeoutError as FutureTimeoutError
from typing import Any

logger = logging.getLogger(__name__)


class RunawayLoopError(Exception):
    """Exception raised when an infinite or runaway loop is detected."""

    pass


class TimeoutMonitor:
    """Monitors tasks and processes to prevent runaway executions."""

    def __init__(self, default_timeout: float = 600.0):
        """Initialize the TimeoutMonitor.

        Args:
            default_timeout: Default timeout in seconds (default 10 minutes).
        """
        self.default_timeout = default_timeout

    def run_with_timeout(
        self,
        func: Callable[..., Any],
        *args: Any,
        timeout_seconds: float | None = None,
        **kwargs: Any,
    ) -> Any:
        """Execute a callable inside a thread pool with a timeout.

        If the timeout is exceeded, cancels the future and raises TimeoutError.
        """
        timeout = (
            timeout_seconds
            if timeout_seconds is not None
            else self.default_timeout
        )
        with ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(func, *args, **kwargs)
            try:
                return future.result(timeout=timeout)
            except FutureTimeoutError as exc:
                future.cancel()
                logger.error("Task timed out after %s seconds.", timeout)
                raise TimeoutError(
                    f"Task timed out after {timeout} seconds."
                ) from exc

    def run_process_with_timeout(
        self,
        cmd_args: list[str],
        timeout_seconds: float | None = None,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
    ) -> tuple[int, str, str]:
        """Run a subprocess with a timeout.

        If the process times out, terminates or kills it and raises TimeoutError.

        Returns:
            (exit_code, stdout, stderr)
        """
        timeout = (
            timeout_seconds
            if timeout_seconds is not None
            else self.default_timeout
        )
        logger.info("Running command: %s with timeout %ss", cmd_args, timeout)

        proc = subprocess.Popen(
            cmd_args,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=cwd,
            env=env,
            text=True,
        )

        try:
            stdout, stderr = proc.communicate(timeout=timeout)
            return proc.returncode, stdout, stderr
        except subprocess.TimeoutExpired as exc:
            logger.warning(
                "Subprocess timed out after %s seconds. Terminating...", timeout
            )
            proc.terminate()
            try:
                # Wait briefly for termination
                stdout, stderr = proc.communicate(timeout=2.0)
            except subprocess.TimeoutExpired:
                logger.warning("Subprocess failed to terminate. Killing...")
                proc.kill()
                stdout, stderr = proc.communicate()
            raise TimeoutError(
                f"Subprocess timed out after {timeout} seconds."
            ) from exc


class RunawayDetector:
    """Tracks action history to detect infinite tool/command execution loops."""

    def __init__(self, max_consecutive_failures: int = 5):
        self.max_consecutive_failures = max_consecutive_failures
        # Map of action_key -> list of (timestamp, success_bool)
        self.history: dict[str, list[tuple[float, bool]]] = {}

    def record_action(self, action_key: str, success: bool) -> None:
        """Record a command or tool run.

        If it fails consecutively more than the threshold, raises RunawayLoopError.
        """
        if action_key not in self.history:
            self.history[action_key] = []

        self.history[action_key].append((time.time(), success))

        # Check consecutive failures
        recent = self.history[action_key][-self.max_consecutive_failures :]
        if len(recent) == self.max_consecutive_failures and all(
            not item[1] for item in recent
        ):
            logger.error(
                "Runaway loop detected! Action '%s' failed %s times consecutively.",
                action_key,
                self.max_consecutive_failures,
            )
            raise RunawayLoopError(
                f"Runaway loop detected: Action '{action_key}' failed "
                f"{self.max_consecutive_failures} times consecutively. Force stopping execution."
            )
