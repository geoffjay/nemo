#!/usr/bin/env python3
"""
Claude Code PreToolUse hook that blocks destructive commands.

Ported from the OpenCode plugin at hooks/destructive-protection.ts.
Reads JSON from stdin, checks for dangerous patterns, and outputs
a JSON denial decision if a destructive command is detected.

Configure in .claude/settings.json:
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "python3 hooks/destructive-protection.py"
          }
        ]
      }
    ]
  }
}
"""

import json
import re
import sys

# Destructive command patterns that should be blocked.
# Each tuple is (compiled_regex, human-readable name).
DESTRUCTIVE_PATTERNS = [
    # File/directory deletion
    (re.compile(r"\brm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?.*"), "rm (force delete)"),
    (re.compile(r"\brm\s+-[a-zA-Z]*r[a-zA-Z]*"), "rm -r (recursive delete)"),
    (re.compile(r"\brmdir\b"), "rmdir"),
    (re.compile(r"\bunlink\b"), "unlink"),
    # Git destructive operations
    (re.compile(r"\bgit\s+push\s+.*--force\b"), "git push --force"),
    (re.compile(r"\bgit\s+push\s+-f\b"), "git push -f"),
    (re.compile(r"\bgit\s+reset\s+--hard\b"), "git reset --hard"),
    (re.compile(r"\bgit\s+clean\s+-[a-zA-Z]*f"), "git clean -f"),
    (re.compile(r"\bgit\s+checkout\s+--\s+\."), "git checkout -- ."),
    (re.compile(r"\bgit\s+stash\s+drop\b"), "git stash drop"),
    (re.compile(r"\bgit\s+branch\s+-[dD]\b"), "git branch delete"),
    (re.compile(r"\bgit\s+rebase\b"), "git rebase"),
    # Database operations
    (re.compile(r"\bDROP\s+(DATABASE|TABLE|INDEX|SCHEMA)\b", re.IGNORECASE), "DROP database object"),
    (re.compile(r"\bTRUNCATE\s+TABLE\b", re.IGNORECASE), "TRUNCATE TABLE"),
    (re.compile(r"\bDELETE\s+FROM\b.*(?!WHERE)", re.IGNORECASE), "DELETE without WHERE"),
    # System-level destructive commands
    (re.compile(r"\bmkfs\b"), "mkfs (format filesystem)"),
    (re.compile(r"\bdd\s+.*of=/dev/"), "dd to device"),
    (re.compile(r"\bshred\b"), "shred"),
    (re.compile(r"\bwipe\b"), "wipe"),
    # Process/service disruption
    (re.compile(r"\bkillall\b"), "killall"),
    (re.compile(r"\bpkill\b"), "pkill"),
    (re.compile(r"\bkill\s+-9\b"), "kill -9"),
    (re.compile(r"\bsystemctl\s+(stop|disable|mask)\b"), "systemctl stop/disable"),
    (re.compile(r"\bservice\s+\S+\s+stop\b"), "service stop"),
    # Container/infrastructure destruction
    (re.compile(r"\bdocker\s+rm\b"), "docker rm"),
    (re.compile(r"\bdocker\s+rmi\b"), "docker rmi"),
    (re.compile(r"\bdocker\s+system\s+prune\b"), "docker system prune"),
    (re.compile(r"\bdocker-compose\s+down\s+-v\b"), "docker-compose down -v"),
    (re.compile(r"\bkubectl\s+delete\b"), "kubectl delete"),
    # Dangerous redirects/overwrites
    (re.compile(r">\s*/dev/(sd|hd|nvme)"), "redirect to disk device"),
    (re.compile(r":\s*>\s*\S+"), "truncate file with :>"),
    # Chmod/chown dangers
    (re.compile(r"\bchmod\s+(-R\s+)?777\b"), "chmod 777"),
    (re.compile(r"\bchown\s+-R\s+.*\s+/"), "recursive chown on root paths"),
    # npm/package manager destructive
    (re.compile(r"\bnpm\s+cache\s+clean\s+--force\b"), "npm cache clean --force"),
]

# Paths that are especially dangerous to modify.
PROTECTED_PATHS = [
    re.compile(r"^/$"),
    re.compile(r"^/etc/"),
    re.compile(r"^/usr/"),
    re.compile(r"^/bin/"),
    re.compile(r"^/sbin/"),
    re.compile(r"^/boot/"),
    re.compile(r"^/var/lib/"),
    re.compile(r"^~/\.(bash|zsh|profile)"),
    re.compile(r"^/home/[^/]+/\.(bash|zsh|profile)"),
]


def matches_destructive_pattern(command: str) -> tuple[bool, str] | None:
    """Check if a command matches any destructive pattern."""
    for pattern, name in DESTRUCTIVE_PATTERNS:
        if pattern.search(command):
            return (True, name)
    return None


def targets_protected_path(command: str) -> str | None:
    """Check if a command targets protected paths."""
    for path_pattern in PROTECTED_PATHS:
        match = path_pattern.search(command)
        if match:
            return match.group(0)
    return None


def deny(reason: str) -> None:
    """Output a JSON denial and exit."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def main() -> None:
    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    tool_name = hook_input.get("tool_name", "")
    tool_input = hook_input.get("tool_input", {})

    # Check bash/shell commands for destructive patterns
    if tool_name in ("Bash", "bash", "shell"):
        command = tool_input.get("command", "")
        if not isinstance(command, str) or not command:
            sys.exit(0)

        result = matches_destructive_pattern(command)
        if result:
            deny(
                f'Blocked destructive command: "{result[1]}"\n'
                f"Command: {command}\n\n"
                "This command could cause data loss or system damage. "
                "If you're sure you want to run this, ask the user to execute it manually."
            )

        protected = targets_protected_path(command)
        if protected:
            deny(
                f"Blocked command targeting protected path: {protected}\n"
                f"Command: {command}\n\n"
                "Modifying system paths can cause serious problems. "
                "If this is intentional, ask the user to execute it manually."
            )

    # Check write tool targeting sensitive files
    if tool_name in ("Write", "write"):
        file_path = tool_input.get("file_path", "") or tool_input.get("filePath", "")
        if isinstance(file_path, str) and file_path:
            protected = targets_protected_path(file_path)
            if protected:
                deny(
                    f"Blocked write to protected path: {protected}\n"
                    f"Path: {file_path}\n\n"
                    "Writing to system paths can cause serious problems."
                )

    # Command is safe — allow
    sys.exit(0)


if __name__ == "__main__":
    main()
