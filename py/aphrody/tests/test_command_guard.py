from aphrody.command_guard import CommandGuard, normalize_command


def test_normalize_command():
    # ANSI escape code stripping
    assert normalize_command("\x1b[31mhello\x1b[0m") == "hello"
    # Null byte stripping
    assert normalize_command("hello\x00world") == "helloworld"
    # NFKC Unicode normalization
    # Fullwidth latin characters normalized to standard ASCII latin characters
    assert normalize_command("ｈｅｌｌｏ") == "hello"


def test_command_guard_hardline():
    guard = CommandGuard()

    # rm -rf /
    allowed, reason = guard.verify_command("rm -rf /")
    assert not allowed
    assert "recursive delete of root filesystem" in reason

    # dd to block device
    allowed, reason = guard.verify_command("dd if=/dev/zero of=/dev/sda")
    assert not allowed
    assert "dd to raw block device" in reason

    # shutdown
    allowed, reason = guard.verify_command("shutdown -h now")
    assert not allowed
    assert "system shutdown/reboot" in reason


def test_command_guard_dangerous():
    guard = CommandGuard()

    # rm -r folder
    allowed, reason = guard.verify_command("rm -r my_folder")
    assert not allowed
    assert "recursive delete" in reason

    # chmod 777 file
    allowed, reason = guard.verify_command("chmod 777 script.sh")
    assert not allowed
    assert "world/other-writable permissions" in reason

    # git reset --hard
    allowed, reason = guard.verify_command("git reset --hard")
    assert not allowed
    assert "git reset --hard" in reason


def test_command_guard_smart_approve():
    # Setup mock smart approval callback
    def mock_approve(cmd, desc):
        if "safe_cmd" in cmd:
            return "approve"
        return "deny"

    guard = CommandGuard(smart_approve_cb=mock_approve)

    # Flagged command but containing safe_cmd - should be approved/allowed
    allowed, reason = guard.verify_command("rm -r safe_cmd_folder")
    assert allowed
    assert reason == ""

    # Flagged command without safe_cmd - should be denied/blocked
    allowed, reason = guard.verify_command("rm -r risky_folder")
    assert not allowed
    assert "recursive delete" in reason
