---
type: Plan
title: Scope nested routers inside SFCs
description: When a `<router>` is nested inside an SFC `<template>`, id-scoping renames its `id` per instance but leaves `<nav-link router="…">` and Rhai `navigate("…")` targets pointing at the stale id. Fix via a static attribute rewrite plus instance-relative runtime resolution.
tags: [sfc, routing, runtime, planning]
timestamp: 2026-07-30T00:00:00Z
---

# Scope nested routers inside SFCs

Nesting a `<router>` inside an SFC `<template>` is currently a **latent break**:
the router renders, but navigation to it silently targets a router id that no
longer exists. This plan closes that gap. It was flagged as a follow-up in both
[single-file components](sfc-components.md) and [page router](page-router.md)
("owned by whichever plan lands second, plus one test") but was never
implemented — both landed without it, and nothing exercises it yet because no
example nests a router in an SFC.

# The bug

SFC instances scope their template-owned child ids per instance:
`scope_template_children` (`runtime.rs:1947`) / `scope_owned_descendants`
(`:1998`) rename each owned component's **map key** to
`format!("{}_{}", parent_id, id)` (`:1970`, `:2016`). So a nested
`<router id="main">` inside SFC `foo` becomes `foo_main` for that instance.

But scoping only touches the **map key** — never attribute *values* or script
*bodies*. Router state is keyed by router id, and two things still reference the
old id:

1. **`<nav-link router="main">`** — the attribute is read raw at render time
   (`app.rs:1228`, `containers/router.rs:149` `.get("router")`); the value `main`
   is never rewritten, so the nav-link points at a router id that doesn't exist
   under the scoped instance.
2. **`navigate("main", …)` / `back("main")` / `forward("main")`** — these route by
   the literal id passed in (`NavIntent` at `runtime.rs:45`, applied at
   `:1148`). They live inside the SFC's Rhai `<script>`, which is never scanned or
   rewritten.

`rewrite_sfc_handlers` (`:2258`) — the compile-time rewrite the KB pointed at as
the model — only rewrites `on_*` keys to `sfc:<tag>::<fn>`; it does nothing for
`router=` and (being a JSON-tree pass) could not touch Rhai strings anyway.

# Key insight: this is two mechanisms, not one

The KB glossed this as "extend the handler rewrite to `router=`/nav targets." It
is not one rewrite — the two references live in different places and, critically,
the scoped id is **per-instance** while the SFC `<script>` is **shared across all
instances** (loaded once under `sfc:<tag>`). So the two halves need different
fixes:

## Mechanism A — static attribute rewrite (nav-link `router=`), at scope time

The attribute case *can* be rewritten statically, but **not** in the
compile-time SFC pass (`rewrite_sfc_handlers`, `:2538-2554`) — the instance
prefix isn't known there. It must happen where the prefix is actually applied:
inside `scope_owned_descendants` / `scope_template_children`, in the same loop
that renames ids.

The rule mirrors id renaming exactly: **a reference to an owned id is scoped the
same way the id itself is.** When scoping a child, if it carries a `router`
attribute whose value is in `owned_ids`, prefix that value with `parent_id` too
(`main` → `foo_main`), so the nav-link tracks the router it names. This requires
the scope functions to inspect each child's own attributes, not just recurse into
the `component` map. Keep it general (any owned-id-referencing attribute) but
scope v1 to `router=` on nav-links.

*Consideration:* a `<nav-link>` with **no** `router=` targets the primary router;
inside an SFC that default is ambiguous. v1 can require an explicit `router=` on
nav-links authored inside an SFC (warn if omitted), deferring "primary router
within an SFC subtree" as a separate question.

## Mechanism B — instance-relative runtime resolution (Rhai `navigate`)

The Rhai case **cannot** be statically rewritten: one `sfc:<tag>` script serves
every instance, so it cannot hardcode `foo_main` vs `bar_main`. The fix is to
resolve the router id **at call time, relative to the calling instance**.

An SFC-scoped handler already receives its instance-scoped `component_id` (e.g.
`foo_labeled_button`), so the instance prefix is derivable. The runtime should,
for the duration of an SFC handler call, know the caller's instance prefix, and
`navigate`/`back`/`forward` should resolve their router argument as: **try
`<prefix>_<arg>` in the router registry first, fall back to `<arg>`** (global).
Then a bare `navigate("main")` authored in an SFC script "just works" for every
instance with no API change, and a top-level `navigate("main")` is unaffected.

Wiring sketch: stash a `current_instance_prefix` on the nav-resolution context
when firing an SFC-scoped handler (the prefix is the leading segment of the
scoped `component_id`); consult it in the `navigate`/`back`/`forward` host fns
before looking up / enqueuing the `NavIntent` (`runtime.rs:45`, `:1148`). No new
Rhai surface, no per-instance script.

# Scope / non-goals

* **v1 targets one level** — a router directly owned by an SFC template. Routers
  nested through *further* SFCs compose by the same prefix rule (each level adds
  its prefix) but should get an explicit test rather than being assumed.
* **Reverse case already works** — an SFC used *inside* a `<route>` body needs
  nothing: the SFC is expanded to built-ins before the router renders the route.
  (Recorded in [sfc-components.md](sfc-components.md) and
  [routing](../patterns/routing.md); the "nested routers work for free" note in
  routing.md is about `<router>`-in-`<route>`, **not** this SFC-template case.)
* **No change to standalone routers** — global `navigate()` / top-level nav-links
  are untouched; both fixes are gated on an owned-id / instance-prefix match.

# Critical files

| File | Role |
|---|---|
| `crates/nemo/src/runtime.rs` | `scope_owned_descendants` (`:1998`) / `scope_template_children` (`:1947`) — add owned `router=` attribute rewrite (Mechanism A); `navigate`/`back`/`forward` host fns + `NavIntent` (`:45`, `:1148`) — instance-relative resolution (Mechanism B); set the caller prefix when firing an SFC handler |
| `crates/nemo/src/containers/router.rs` | `.get("router")` read (`:149`) — no change if the attribute is pre-scoped, but confirm it reads the rewritten value |
| `crates/nemo/src/app.rs` | nav-link `router` read (`:1228`) — same confirmation |

# Verification

* **Unit (runtime):** an SFC whose template nests `<router id="main">` +
  `<nav-link router="main">`, instantiated **twice**; assert each instance's
  router id scopes to `<instance>_main` **and** its nav-link `router` attribute is
  rewritten to match (Mechanism A), through `parse_layout_config` — the
  multi-instance no-collision shape the SFC tests already use.
* **End-to-end (Rhai):** an SFC `<script>` handler calling `navigate("main", …)`;
  two instances; assert each navigates **its own** nested router, not the other's
  and not a global `main` (Mechanism B). Mirror
  `test_task_list_handlers_end_to_end` style with a mock `PluginContext`.
* **Regression:** standalone `examples/router/` still navigates (global path
  unaffected); a top-level `navigate("main")` with no SFC scope still resolves
  globally.
* **Optional example:** extend `examples/sfc/` (or a small new example) with a
  router-bearing SFC used twice, validated `--strict` and run in a live window
  (`nemo-run`; local builds need
  `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`).

# Relationship to other plans

* Closes the open follow-up in [single-file components](sfc-components.md#relationship-to-the-page-router-plan)
  and [page router](page-router.md#relationship-to-the-sfc-plan) — those sections
  now point here.
* Independent of the [raw-text `.nemo` parser](sfc-raw-text-parser.md) and the
  [build system](build-system.md) (a built `dist/` contains already-scoped ids, so
  Mechanism A's rewrite is baked in at build time; Mechanism B is a runtime host-fn
  concern either way).

# Knowledgebase updates required when implemented

* [Routing](../patterns/routing.md) — document that a router nested in an SFC is
  scoped per instance and that nav-link/`navigate()` targets resolve to the
  instance's router; correct the "for free" nested-router note to distinguish the
  two nesting cases.
* [Single-file components](../patterns/single-file-components.md) — note router
  nesting under "Rules that bite" (explicit `router=` required inside an SFC).
* [Roadmap](roadmap.md) and this plan — mark implemented.
