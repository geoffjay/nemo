#!/usr/bin/env python3
"""
Claude Code SessionStart hook that injects the knowledge base index.

The nemo knowledge base (docs/knowledgebase/, OKF v0.1) carries the project's
decisions, patterns, and conventions. This hook injects docs/knowledgebase/index.md
as context at the start of every session (startup, resume, clear, compact) so an
agent always has the KB map and the consult/update policy in hand — independent of
whether CLAUDE.md gets picked up.

The index is a compact manifest (descriptions, not full docs); the agent reads
individual concept/decision/pattern docs on demand.

Configured under "SessionStart" in .claude/settings.json.
"""

import json
import os
import sys

INDEX_REL = os.path.join("docs", "knowledgebase", "index.md")


def project_dir(hook_input: dict) -> str:
    """Resolve the project root from the env var, falling back to the payload cwd."""
    return os.environ.get("CLAUDE_PROJECT_DIR") or hook_input.get("cwd") or os.getcwd()


def main() -> int:
    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        hook_input = {}

    index_path = os.path.join(project_dir(hook_input), INDEX_REL)
    try:
        with open(index_path, encoding="utf-8") as f:
            index = f.read()
    except OSError:
        # No KB in this checkout: nothing to inject, don't get in the way.
        return 0

    context = (
        "# nemo knowledge base (injected)\n\n"
        "The project keeps a knowledge base at docs/knowledgebase/. Consult the "
        "relevant entries before working on a task and update them when a change "
        "makes an entry wrong or leaves a new fact unrecorded — the policy and the "
        "full map are below. Read individual docs on demand.\n\n"
        "---\n\n" + index
    )

    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": context,
        }
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
