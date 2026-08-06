/**
 * nemo knowledge-base hooks for oh-my-pi.
 *
 * Port of the Claude Code hooks at `.claude/hooks/kb-inject.py` and
 * `.claude/hooks/kb-reminder.py` to the omp extension event model:
 *
 *   - `session_start` + `before_agent_start`: inject `docs/knowledgebase/index.md`
 *     as context at the start of every session (startup, resume, switch), so the
 *     agent always has the KB map and the consult/update policy in hand —
 *     independent of whether `CLAUDE.md`/`AGENTS.md` gets picked up.
 *   - `tool_result` (write|edit): when a KB-relevant source file is edited,
 *     record it and, once per session, inject an inline reminder to consider a
 *     KB update. When a `docs/knowledgebase/` file is edited, record that the KB
 *     was touched.
 *   - `session_stop`: as a backstop, if the session changed KB-relevant files but
 *     never touched the KB, block the stop exactly once with a reminder.
 *
 * This never hard-blocks an edit — enforcing "did you look" is brittle. It only
 * injects context and, at most once, asks the agent to reconsider at the end.
 * All failures are fail-open so the KB policy can never wedge real work.
 *
 * Because the extension runs in-process for the whole session, per-session state
 * lives in module-level variables (reset on `session_start`), replacing the
 * temp-file markers the stateless Claude Code Python hooks needed.
 */

import type { ExtensionAPI } from "@oh-my-pi/pi-coding-agent";
import * as fs from "node:fs";
import * as path from "node:path";

const INDEX_REL = path.join("docs", "knowledgebase", "index.md");
const KB_PREFIX = "docs/knowledgebase/";

// Repo-relative path patterns that map to KB topics worth keeping in sync.
// (compiled regex, human-readable hint shown in the reminder)
const TRIGGERS: ReadonlyArray<{ pattern: RegExp; hint: string }> = [
  { pattern: /^crates\/nemo-config\//, hint: "config parse/resolve/validate → concepts/configuration.md" },
  { pattern: /^crates\/nemo-layout\//, hint: "layout building / bindings → concepts/configuration.md + concepts/architecture.md" },
  { pattern: /^crates\/nemo-registry\//, hint: "component registration/schemas → concepts/components.md + patterns/four-file-component-workflow.md" },
  { pattern: /^crates\/nemo\/src\/components\//, hint: "components → concepts/components.md + patterns/four-file-component-workflow.md" },
  { pattern: /^crates\/nemo\/src\/(app|runtime|main)\.rs$/, hint: "render dispatch / runtime / startup → concepts/architecture.md" },
  { pattern: /^crates\/nemo-macros\//, hint: "NemoComponent macro → concepts/components.md" },
  { pattern: /^crates\/nemo-data\//, hint: "data sources/transforms repository → concepts/data-flow.md" },
  { pattern: /^crates\/nemo-integration\//, hint: "integration gateway → concepts/data-flow.md" },
  { pattern: /^crates\/nemo-(extension|plugin|plugin-api|wasm|wasm-guest)\//, hint: "extensions/plugins → concepts/extensions.md" },
  { pattern: /^Cargo\.(toml|lock)$/, hint: "gpui git dep is load-bearing → decisions/pin-gpui-git-dep.md" },
  { pattern: /^examples\//, hint: "example apps → plans/roadmap.md if scope shifts" },
];

const KB_INJECT_PREAMBLE =
  "# nemo knowledge base (injected)\n\n" +
  "The project keeps a knowledge base at docs/knowledgebase/. Consult the " +
  "relevant entries before working on a task and update them when a change " +
  "makes an entry wrong or leaves a new fact unrecorded — the policy and the " +
  "full map are below. Read individual docs on demand.\n\n" +
  "---\n\n";

// --- per-session state (reset on session_start) -----------------------------
let projectCwd = "";
let kbIndex: string | null = null;
let injected = false;
let dirty = false; // a KB-relevant source file was changed
let touched = false; // docs/knowledgebase/ was changed
let nudged = false; // inline post-edit nudge fired
let reminded = false; // stop backstop fired
const dirtyHints = new Set<string>();
let lastEditedRel = "";

// --- helpers -----------------------------------------------------------------

function relPath(rawPath: string): string | null {
  if (!rawPath || typeof rawPath !== "string") return null;
  // Strip a copied [path#TAG] hashline wrapper if present.
  const tagMatch = rawPath.match(/^\[(.+?)#[0-9A-F]{4}\](.*)$/);
  let p = tagMatch ? tagMatch[1] + (tagMatch[2] || "") : rawPath;
  p = p.trim();
  if (!p) return null;
  const root = projectCwd ? path.resolve(projectCwd) : process.cwd();
  const abs = path.isAbsolute(p) ? p : path.resolve(root, p);
  try {
    const rel = path.relative(root, abs);
    if (rel.startsWith("..")) return null;
    return rel.replace(/\\/g, "/");
  } catch {
    return null;
  }
}

/** Extract repo-relative paths edited by a `write` or `edit` tool call. */
function editedPaths(event: { toolName: string; input: Record<string, unknown> }): string[] {
  const input = event.input ?? {};
  if (event.toolName === "write") {
    const p = relPath(String(input.path ?? ""));
    return p ? [p] : [];
  }
  if (event.toolName === "edit") {
    // hashline `input` contains one or more `[PATH#TAG]` sections.
    const patch = String(input.input ?? "");
    const out: string[] = [];
    for (const m of patch.matchAll(/\[(.+?)#[0-9A-F]{4}\]/g)) {
      const p = relPath(m[1]);
      if (p && !out.includes(p)) out.push(p);
    }
    return out;
  }
  return [];
}

// --- extension factory -------------------------------------------------------

export default function kbHooks(pi: ExtensionAPI): void {
  pi.setLabel("nemo KB hooks");

  pi.on("session_start", async (_event, ctx) => {
    // Reset per-session state for startup / resume / switch.
    projectCwd = ctx.cwd ?? process.cwd();
    kbIndex = null;
    injected = false;
    dirty = false;
    touched = false;
    nudged = false;
    reminded = false;
    dirtyHints.clear();
    lastEditedRel = "";

    try {
      const indexPath = path.join(projectCwd, INDEX_REL);
      kbIndex = fs.readFileSync(indexPath, "utf-8");
    } catch {
      // No KB in this checkout: nothing to inject, don't get in the way.
      kbIndex = null;
    }
  });

  pi.on("before_agent_start", async () => {
    // Inject the KB index once, on the first agent turn of the session.
    if (kbIndex && !injected) {
      injected = true;
      return {
        message: {
          customType: "kb-inject",
          content: KB_INJECT_PREAMBLE + kbIndex,
          display: false,
          attribution: "user" as const,
        },
      };
    }
    // Once per session, nudge after a KB-relevant edit.
    if (dirty && !nudged) {
      nudged = true;
      const topics = [...dirtyHints].sort().join(", ");
      return {
        message: {
          customType: "kb-nudge",
          content:
            "Knowledge base: you changed a KB-relevant file (" +
            lastEditedRel +
            "). Per docs/knowledgebase/index.md, consider whether this warrants a " +
            "KB update (" +
            topics +
            "). Not every edit needs one — use judgement.",
          display: false,
          attribution: "user" as const,
        },
      };
    }
    return undefined;
  });

  pi.on("tool_result", async (event) => {
    if (event.isError) return;
    const rels = editedPaths(event);
    if (!rels.length) return;

    for (const rel of rels) {
      // Editing the KB itself counts as keeping it current.
      if (rel.startsWith(KB_PREFIX)) {
        touched = true;
        continue;
      }
      for (const t of TRIGGERS) {
        if (t.pattern.test(rel)) {
          dirty = true;
          lastEditedRel = rel;
          dirtyHints.add(t.hint);
        }
      }
    }
  });

  pi.on("session_stop", async (event) => {
    // Don't re-fire inside a stop-hook-triggered continuation.
    if (typeof event === "object" && event !== null && "stop_hook_active" in event) {
      if ((event as { stop_hook_active: unknown }).stop_hook_active) return;
    }

    if (!dirty || touched || reminded) return;

    // Fire the backstop exactly once.
    reminded = true;

    const topics = dirtyHints.size
      ? [...dirtyHints].sort().join(", ")
      : "see docs/knowledgebase/index.md";

    const reason =
      "This session changed KB-relevant files but did not touch " +
      "docs/knowledgebase/. Per the knowledge base policy, update the relevant " +
      "docs now (" +
      topics +
      "), or state briefly why no KB update is needed. This reminder fires only once.";

    return { decision: "block" as const, reason };
  });
}