#!/usr/bin/env python3
"""
Claude Code hook that nudges an agent to keep the knowledge base current.

Registered for two events in .claude/settings.json:

  * PostToolUse (Write|Edit|MultiEdit): when a KB-relevant source file is edited,
    record it and, once per session, inject an inline reminder to consider a KB
    update. When a docs/knowledgebase/ file is edited, record that the KB was
    touched.
  * Stop: as a backstop, if the session changed KB-relevant files but never
    touched the KB, block the stop exactly once with a reminder.

This never hard-blocks an edit — enforcing "did you look" is brittle. It only
injects context and, at most once, asks the agent to reconsider at the end.
All failures are fail-open (exit 0) so the KB policy can never wedge real work.

Per-session state lives under a temp dir keyed by session_id.
"""

import json
import os
import re
import sys
import tempfile

# Repo-relative path patterns that map to KB topics worth keeping in sync.
# (compiled regex, human-readable hint shown in the reminder)
TRIGGERS = [
    (re.compile(r"^crates/nemo-config/"), "config parse/resolve/validate → concepts/configuration.md"),
    (re.compile(r"^crates/nemo-layout/"), "layout building / bindings → concepts/configuration.md + concepts/architecture.md"),
    (re.compile(r"^crates/nemo-registry/"), "component registration/schemas → concepts/components.md + patterns/four-file-component-workflow.md"),
    (re.compile(r"^crates/nemo/src/components/"), "components → concepts/components.md + patterns/four-file-component-workflow.md"),
    (re.compile(r"^crates/nemo/src/(app|runtime|main)\.rs$"), "render dispatch / runtime / startup → concepts/architecture.md"),
    (re.compile(r"^crates/nemo-macros/"), "NemoComponent macro → concepts/components.md"),
    (re.compile(r"^crates/nemo-data/"), "data sources/transforms/repository → concepts/data-flow.md"),
    (re.compile(r"^crates/nemo-integration/"), "integration gateway → concepts/data-flow.md"),
    (re.compile(r"^crates/nemo-(extension|plugin|plugin-api|wasm|wasm-guest)/"), "extensions/plugins → concepts/extensions.md"),
    (re.compile(r"^Cargo\.(toml|lock)$"), "gpui git dep is load-bearing → decisions/pin-gpui-git-dep.md"),
    (re.compile(r"^examples/"), "example apps → plans/roadmap.md if scope shifts"),
]

KB_PREFIX = "docs/knowledgebase/"


def project_dir(hook_input: dict) -> str:
    return os.environ.get("CLAUDE_PROJECT_DIR") or hook_input.get("cwd") or os.getcwd()


def state_dir(session_id: str) -> str:
    d = os.path.join(tempfile.gettempdir(), "nemo-kb-state", session_id or "unknown")
    os.makedirs(d, exist_ok=True)
    return d


def marker(session_id: str, name: str) -> str:
    return os.path.join(state_dir(session_id), name)


def rel_path(hook_input: dict, tool_input: dict) -> str | None:
    """Repo-relative path of the edited file, or None if it can't be determined."""
    raw = tool_input.get("file_path") or tool_input.get("filePath") or ""
    if not isinstance(raw, str) or not raw:
        return None
    root = os.path.abspath(project_dir(hook_input))
    ap = raw if os.path.isabs(raw) else os.path.join(root, raw)
    try:
        rel = os.path.relpath(os.path.abspath(ap), root)
    except ValueError:
        return None
    if rel.startswith(".."):
        return None
    return rel.replace(os.sep, "/")


def handle_post_tool_use(hook_input: dict, session_id: str) -> int:
    rel = rel_path(hook_input, hook_input.get("tool_input", {}))
    if rel is None:
        return 0

    # Editing the KB itself counts as keeping it current.
    if rel.startswith(KB_PREFIX):
        open(marker(session_id, "touched"), "a").close()
        return 0

    hints = [hint for pat, hint in TRIGGERS if pat.search(rel)]
    if not hints:
        return 0

    # Record that a KB-relevant change happened this session.
    with open(marker(session_id, "dirty"), "a", encoding="utf-8") as f:
        for h in hints:
            f.write(h + "\n")

    # Inject the inline nudge at most once per session to avoid noise.
    nudged = marker(session_id, "nudged")
    if os.path.exists(nudged):
        return 0
    open(nudged, "a").close()

    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": (
                "Knowledge base: you changed a KB-relevant file (" + rel + "). "
                "Per docs/knowledgebase/index.md, consider whether this warrants a "
                "KB update (" + hints[0] + "). Not every edit needs one — use judgement."
            ),
        }
    }))
    return 0


def handle_stop(hook_input: dict, session_id: str) -> int:
    # Don't re-fire inside a stop-hook-triggered continuation.
    if hook_input.get("stop_hook_active"):
        return 0

    dirty = os.path.exists(marker(session_id, "dirty"))
    touched = os.path.exists(marker(session_id, "touched"))
    reminded = os.path.exists(marker(session_id, "reminded"))

    if not dirty or touched or reminded:
        return 0

    # Fire the backstop exactly once.
    open(marker(session_id, "reminded"), "a").close()

    topics = ""
    try:
        with open(marker(session_id, "dirty"), encoding="utf-8") as f:
            topics = ", ".join(sorted({line.strip() for line in f if line.strip()}))
    except OSError:
        pass

    reason = (
        "This session changed KB-relevant files but did not touch "
        "docs/knowledgebase/. Per the knowledge base policy, update the relevant "
        "docs now (" + (topics or "see docs/knowledgebase/index.md") + "), or state "
        "briefly why no KB update is needed. This reminder fires only once."
    )
    print(json.dumps({"decision": "block", "reason": reason}))
    return 0


def main() -> int:
    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        return 0

    session_id = hook_input.get("session_id", "")
    event = hook_input.get("hook_event_name", "")

    try:
        if event == "PostToolUse":
            return handle_post_tool_use(hook_input, session_id)
        if event == "Stop":
            return handle_stop(hook_input, session_id)
    except Exception:
        # Fail open: the KB policy must never block real work.
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
