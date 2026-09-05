# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""Security guard enforcing headless blocklists on shell commands."""

import re
import unicodedata
from collections.abc import Callable

# ANSI escape sequence regex from Hermes tools.ansi_strip
_ANSI_ESCAPE_RE = re.compile(
    r"\x1b"
    r"(?:"
    r"\[[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]"  # CSI sequence
    r"|\][\s\S]*?(?:\x07|\x1b\\)"  # OSC (BEL or ST terminator)
    r"|[PX^_][\s\S]*?(?:\x1b\\)"  # DCS/SOS/PM/APC strings
    r"|[\x20-\x2f]+[\x30-\x7e]"  # nF escape sequences
    r"|[\x30-\x7e]"  # Fp/Fe/Fs single-byte
    r")"
    r"|\x9b[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]"  # 8-bit CSI
    r"|\x9d[\s\S]*?(?:\x07|\x9c)"  # 8-bit OSC
    r"|[\x80-\x9f]",  # Other 8-bit C1 controls
    re.DOTALL,
)
_HAS_ESCAPE = re.compile(r"[\x1b\x80-\x9f]")


def strip_ansi(text: str) -> str:
    """Remove ANSI escape sequences from text."""
    if not text or not _HAS_ESCAPE.search(text):
        return text
    return _ANSI_ESCAPE_RE.sub("", text)


def normalize_command(command: str) -> str:
    """Normalize command string to prevent bypasses."""
    command = strip_ansi(command)
    command = command.replace("\x00", "")
    command = unicodedata.normalize("NFKC", command)
    return command


# Hardline unconditional blocks
HARDLINE_PATTERNS = [
    (
        r"\brm\s+(-[^\s]*\s+)*(/|/\*|/ /\*)(\s|$)",
        "recursive delete of root filesystem",
    ),
    (
        r"\brm\s+(-[^\s]*\s+)*(/home|/home/\*|/root|/root/\*|/etc|/etc/\*|/usr|/usr/\*|/var|/var/\*|/bin|/bin/\*|/sbin|/sbin/\*|/boot|/boot/\*|/lib|/lib/\*)(\s|$)",
        "recursive delete of system directory",
    ),
    (
        r"\brm\s+(-[^\s]*\s+)*(~|\$HOME)(/?|/\*)?(\s|$)",
        "recursive delete of home directory",
    ),
    (r"\bmkfs(\.[a-z0-9]+)?\b", "format filesystem (mkfs)"),
    (
        r"\bdd\b[^\n]*\bof=/dev/(sd|nvme|hd|mmcblk|vd|xvd)[a-z0-9]*",
        "dd to raw block device",
    ),
    (
        r">\s*/dev/(sd|nvme|hd|mmcblk|vd|xvd)[a-z0-9]*\b",
        "redirect to raw block device",
    ),
    (r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:", "fork bomb"),
    (r"\bkill\s+(-[^\s]+\s+)*-1\b", "kill all processes"),
    (
        r"(?:^|[;&|\n`]|\$\()\s*(?:sudo\s+(?:-[^\s]+\s+)*)?(?:env\s+(?:\w+=\S*\s+)*)?(?:(?:exec|nohup|setsid|time)\s+)*\s*(shutdown|reboot|halt|poweroff)\b",
        "system shutdown/reboot",
    ),
    (
        r"(?:^|[;&|\n`]|\$\()\s*(?:sudo\s+(?:-[^\s]+\s+)*)?(?:env\s+(?:\w+=\S*\s+)*)?(?:(?:exec|nohup|setsid|time)\s+)*\s*init\s+[06]\b",
        "init 0/6 (shutdown/reboot)",
    ),
    (
        r"(?:^|[;&|\n`]|\$\()\s*(?:sudo\s+(?:-[^\s]+\s+)*)?(?:env\s+(?:\w+=\S*\s+)*)?(?:(?:exec|nohup|setsid|time)\s+)*\s*systemctl\s+(poweroff|reboot|halt|kexec)\b",
        "systemctl poweroff/reboot",
    ),
    (
        r"(?:^|[;&|\n`]|\$\()\s*(?:sudo\s+(?:-[^\s]+\s+)*)?(?:env\s+(?:\w+=\S*\s+)*)?(?:(?:exec|nohup|setsid|time)\s+)*\s*telinit\s+[06]\b",
        "telinit 0/6 (shutdown/reboot)",
    ),
]

# Dangerous conditional patterns
DANGEROUS_PATTERNS = [
    (r"\brm\s+(-[^\s]*\s+)*/", "delete in root path"),
    (r"\brm\s+-[^\s]*r", "recursive delete"),
    (r"\brm\s+--recursive\b", "recursive delete (long flag)"),
    (
        r"\bchmod\s+(-[^\s]*\s+)*(777|666|o\+[rwx]*w|a\+[rwx]*w)\b",
        "world/other-writable permissions",
    ),
    (
        r"\bchmod\s+--recursive\b.*(777|666|o\+[rwx]*w|a\+[rwx]*w)",
        "recursive world/other-writable (long flag)",
    ),
    (r"\bchown\s+(-[^\s]*)?R\s+root", "recursive chown to root"),
    (r"\bchown\s+--recursive\b.*root", "recursive chown to root (long flag)"),
    (r"\bmkfs\b", "format filesystem"),
    (r"\bdd\s+.*if=", "disk copy"),
    (r">\s*/dev/sd", "write to block device"),
    (r"\bDROP\s+(TABLE|DATABASE)\b", "SQL DROP"),
    (r"\bDELETE\s+FROM\b(?![^\n]*\bWHERE\b)", "SQL DELETE without WHERE"),
    (r"\bTRUNCATE\s+(TABLE)?\s*\w", "SQL TRUNCATE"),
    (
        r">\s*(?:/etc/|/private/(?:etc|var|tmp|home)/)",
        "overwrite system config",
    ),
    (
        r"\bsystemctl\s+(-[^\s]+\s+)*(stop|restart|disable|mask)\b",
        "stop/restart system service",
    ),
    (r"\bkill\s+-9\s+-1\b", "kill all processes"),
    (r"\bpkill\s+-9\b", "force kill processes"),
    (
        r"\bkillall\s+(-[^\s]*\s+)*-(9|KILL|SIGKILL)\b",
        "force kill processes (killall -KILL)",
    ),
    (
        r"\bkillall\s+(-[^\s]*\s+)*-s\s+(KILL|SIGKILL|9)\b",
        "force kill processes (killall -s KILL)",
    ),
    (r"\bkillall\s+(-[^\s]*\s+)*-r\b", "kill processes by regex (killall -r)"),
    (r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:", "fork bomb"),
    (r"\b(bash|sh|zsh|ksh)\s+-[^\s]*c(\s+|$)", "shell command via -c/-lc flag"),
    (
        r"\b(python[23]?|perl|ruby|node)\s+-[ec]\s+",
        "script execution via -e/-c flag",
    ),
    (r"\b(curl|wget)\b.*\|\s*(ba)?sh\b", "pipe remote content to shell"),
    (
        r"\b(bash|sh|zsh|ksh)\s+<\s*<?\s*\(\s*(curl|wget)\b",
        "execute remote script via process substitution",
    ),
    (
        r'\btee\b.*["\']?(?:(?:/etc/|/private/(?:etc|var|tmp|home)/)|/dev/sd|(?:~|\$home|\$\{home\})/\.ssh(?:/|$)|(?:~\/\.hermes/|(?:\$home|\$\{home\})/\.hermes/|(?:\$hermes_home|\$\{hermes_home\})/)\.env\b|(?:~|\$home|\$\{home\})/\.(?:bashrc|zshrc|profile|bash_profile|zprofile)\b|(?:~|\$home|\$\{home\})/\.(?:netrc|pgpass|npmrc|pypirc)\b)',
        "overwrite system file via tee",
    ),
    (
        r'>>?\s*["\']?(?:(?:/etc/|/private/(?:etc|var|tmp|home)/)|/dev/sd|(?:~|\$home|\$\{home\})/\.ssh(?:/|$)|(?:~\/\.hermes/|(?:\$home|\$\{home\})/\.hermes/|(?:\$hermes_home|\$\{hermes_home\})/)\.env\b|(?:~|\$home|\$\{home\})/\.(?:bashrc|zshrc|profile|bash_profile|zprofile)\b|(?:~|\$home|\$\{home\})/\.(?:netrc|pgpass|npmrc|pypirc)\b)',
        "overwrite system file via redirection",
    ),
    (
        r'\btee\b.*["\']?(?:(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*\.env(?:\.[^/\s"\'`]+)*)|(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*config\.yaml))["\']?(?:\s*(?:&&|\|\||;).*)?$',
        "overwrite project env/config via tee",
    ),
    (
        r'>>?\s*["\']?(?:(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*\.env(?:\.[^/\s"\'`]+)*)|(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*config\.yaml))["\']?(?:\s*(?:&&|\|\||;).*)?$',
        "overwrite project env/config via redirection",
    ),
    (r"\bxargs\s+.*\brm\b", "xargs with rm"),
    (r"\bfind\b.*-exec(?:dir)?\s+(/\S*/)?rm\b", "find -exec/-execdir rm"),
    (r"\bfind\b.*-delete\b", "find -delete"),
    (
        r"\b(pkill|killall)\b.*\b(hermes|gateway|cli\.py)\b",
        "kill process (self-termination)",
    ),
    (r"\bkill\b.*\$\(\s*pgrep\b", "kill process via pgrep expansion"),
    (r"\bkill\b.*`\s*pgrep\b", "kill process via backtick pgrep expansion"),
    (
        r"\b(cp|mv|install)\b.*\s(?:/etc/|/private/(?:etc|var|tmp|home)/)",
        "copy/move file into system config path",
    ),
    (
        r'\b(cp|mv|install)\b.*\s["\']?(?:(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*\.env(?:\.[^/\s"\'`]+)*)|(?:(?:/|Running\s+)?(?:\.{1,2}/)?(?:[^\s/"\'`]+/)*config\.yaml))["\']?(?:\s*(?:&&|\|\||;).*)?$',
        "overwrite project env/config file",
    ),
    (
        r"\bsed\s+-[^\s]*i.*\s(?:/etc/|/private/(?:etc|var|tmp|home)/)",
        "in-place edit of system config",
    ),
    (
        r"\bsed\s+--in-place\b.*\s(?:/etc/|/private/(?:etc|var|tmp|home)/)",
        "in-place edit of system config (long flag)",
    ),
    (r"\b(python[23]?|perl|ruby|node)\s+<<", "script execution via heredoc"),
    (
        r"\bgit\s+reset\s+--hard\b",
        "git reset --hard (destroys uncommitted changes)",
    ),
    (r"\bgit\s+push\b.*--force\b", "git force push"),
    (r"\bgit\s+push\b.*-f\b", "git force push short flag"),
    (r"\bgit\s+clean\s+-[^\s]*f", "git clean with force"),
    (r"\bgit\s+branch\s+-D\b", "git branch force delete"),
    (
        r"\bchmod\s+\+x\b.*[;&|]+\s*\./",
        "chmod +x followed by immediate execution",
    ),
    (
        r"\bsudo\b[^;|&\\n]*?\s+(?:-s\b|--stdin\b|-a\b|--askpass\b)",
        "sudo with privilege flag",
    ),
    (
        r"\bsudo\b[^;|&\\n]*?\s+-[a-z]*[sa][a-z]*\b",
        "sudo with combined-flag privilege escalation",
    ),
]

HARDLINE_PATTERNS_COMPILED = [
    (re.compile(p, re.IGNORECASE), desc) for p, desc in HARDLINE_PATTERNS
]
DANGEROUS_PATTERNS_COMPILED = [
    (re.compile(p, re.IGNORECASE), desc) for p, desc in DANGEROUS_PATTERNS
]


class SecurityError(Exception):
    """Exception raised when a command is rejected by CommandGuard."""

    pass


class CommandGuard:
    """Security guard that enforces a headless blocklist on shell commands."""

    def __init__(
        self, smart_approve_cb: Callable[[str, str], str] | None = None
    ):
        """Initialize the CommandGuard.

        Args:
            smart_approve_cb: An optional callback taking (command, description)
                             and returning "approve", "deny", or "escalate".
        """
        self.smart_approve_cb = smart_approve_cb

    def verify_command(self, command: str) -> tuple[bool, str]:
        """Check command against blocklists.

        Returns:
            (allowed, reason_or_empty_str)
        """
        norm_cmd = normalize_command(command)

        # 1. Check hardline unconditional blocks
        for pat_re, desc in HARDLINE_PATTERNS_COMPILED:
            if pat_re.search(norm_cmd):
                return (
                    False,
                    f"SECURITY DENIED: The command matches hardline blocked pattern '{desc}'. Execution blocked.",
                )

        # 2. Check dangerous conditional patterns
        for pat_re, desc in DANGEROUS_PATTERNS_COMPILED:
            if pat_re.search(norm_cmd):
                # Run smart approval if configured
                if self.smart_approve_cb:
                    decision = self.smart_approve_cb(command, desc)
                    if decision == "approve":
                        continue
                return False, (
                    f"SECURITY DENIED: The command matches dangerous pattern '{desc}'. "
                    f"Execution blocked. Choose a safer alternative (e.g., write a Python script "
                    f"using standard libraries instead of raw shell commands)."
                )

        return True, ""
