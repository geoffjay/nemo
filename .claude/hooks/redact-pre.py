#!/usr/bin/env python3

import json, os, re, sys

SECRET_PATTERNS = [
    re.compile(r'(?i)\b[A-Z0-9]{20,}[_-]?[A-Z0-9]{10,}\b'),          # generic long tokens
    re.compile(r'(?i)\b(?:api|secret|token|key|passwd|password)\s*[:=]\s*["\']?([^\s"\']+)'),
    re.compile(r'(?i)sk-[a-z0-9]{20,}'),                              # common key prefix
]

REDACT = "★★★REDACTED★★★"

def scrub(obj):
    if isinstance(obj, dict):
        clean = {}
        for k, v in obj.items():
            lk = k.lower()
            if lk in {"content", "command", "headers", "authorization", "auth", "password"}:
                clean[k] = REDACT
            else:
                clean[k] = scrub(v)
        return clean
    if isinstance(obj, list):
        return [scrub(v) for v in obj]
    if isinstance(obj, str):
        s = obj
        for pat in SECRET_PATTERNS:
            s = pat.sub(REDACT, s)
        return s
    return obj

def main():
    try:
        data = json.load(sys.stdin)
    except Exception as e:
        print(f"Invalid JSON: {e}", file=sys.stderr)
        sys.exit(1)

    tool_name = data.get("tool_name", "")
    tool_input = data.get("tool_input", {})

    # Hard block risky bash patterns
    if tool_name == "Bash":
        cmd = tool_input.get("command", "") or ""
        if re.search(r'\b(curl|wget)\b.*\s(-H|--header)\s.*(authorization|api-key)', cmd, re.I):
            print("Blocking Bash call that would echo auth headers", file=sys.stderr)
            sys.exit(2)  # block

    # Approve benign reads quietly to cut friction
    decision = None
    reason = None
    if tool_name == "Read":
        fp = tool_input.get("file_path", "")
        if fp.endswith((".md", ".mdx", ".txt", ".json")):
            decision = "allow"
            reason = "Documentation read auto approved"

    out = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            **({"permissionDecision": decision, "permissionDecisionReason": reason} if decision else {})
        }
    }
    print(json.dumps(out))
    sys.exit(0)

if __name__ == "__main__":
    main()
