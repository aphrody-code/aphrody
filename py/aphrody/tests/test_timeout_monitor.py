import sys
import time

import pytest
from aphrody.timeout_monitor import (
    RunawayDetector,
    RunawayLoopError,
    TimeoutMonitor,
)


def test_run_with_timeout_success():
    monitor = TimeoutMonitor()

    def fast_fn(x):
        return x + 1

    assert monitor.run_with_timeout(fast_fn, 5, timeout_seconds=1.0) == 6


def test_run_with_timeout_failure():
    monitor = TimeoutMonitor()

    def slow_fn():
        time.sleep(2.0)
        return "done"

    with pytest.raises(TimeoutError) as excinfo:
        monitor.run_with_timeout(slow_fn, timeout_seconds=0.1)
    assert "timed out after 0.1 seconds" in str(excinfo.value)


def test_run_process_with_timeout_success():
    monitor = TimeoutMonitor()
    # Simple python run that succeeds quickly
    ret_code, stdout, _stderr = monitor.run_process_with_timeout(
        [sys.executable, "-c", "print('hello')"], timeout_seconds=2.0
    )
    assert ret_code == 0
    assert stdout.strip() == "hello"


def test_run_process_with_timeout_failure():
    monitor = TimeoutMonitor()
    # Python run that hangs and sleeps for 5 seconds
    with pytest.raises(TimeoutError) as excinfo:
        monitor.run_process_with_timeout(
            [sys.executable, "-c", "import time; time.sleep(5)"],
            timeout_seconds=0.2,
        )
    assert "timed out after 0.2 seconds" in str(excinfo.value)


def test_runaway_detector():
    detector = RunawayDetector(max_consecutive_failures=3)

    # Success records don't trigger anything
    detector.record_action("test_action", True)
    detector.record_action("test_action", False)
    detector.record_action("test_action", True)

    # 1 failure
    detector.record_action("test_action", False)
    # 2 failures
    detector.record_action("test_action", False)
    # 3 failures - should raise RunawayLoopError
    with pytest.raises(RunawayLoopError) as excinfo:
        detector.record_action("test_action", False)

    assert "Runaway loop detected" in str(excinfo.value)
