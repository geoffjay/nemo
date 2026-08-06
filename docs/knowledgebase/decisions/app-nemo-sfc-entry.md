---
type: Decision
title: The application entry is a `.nemo` SFC (not `app.xml`)
description: The root configuration file is `app.nemo` — a single-file component using `<template>`/`<props>`/`<style>`/`<script>`, compiled to a binary format at build time. Supersedes the XML-is-the-config decision for the entry file.
tags: [config, sfc, decision]
timestamp: 2026-08-05T00:00:00Z
---

# Decision

The project entry file is **`app.nemo`** — a single-file component (SFC) that
uses the same `<template>`/`<props>`/`<style>`/`<script>` structure as component
`.nemo` files. It is compiled at build time into a binary format stored under
`dist/` and loaded by the runtime. This **amends and supersedes** [XML is the
configuration format (not HCL)](xml-not-hcl-config.md) for the *entry* file:
component `.nemo` files were already SFCs; the app entry now is too.

# Context

The original decision established XML (`app.xml`) as the single config format.
At the time, the only reuse mechanism was `<templates>`/`<include>` (global,
XML-only), and there was no build layer. Since then, single-file components
(`.nemo` SFCs) were implemented (Phases 0–4), a build system (`nemo build` →
`dist/layout.json`) was added, and the `config-dev-env.md` plan stated the end
goal explicitly: "all configuration files are nemo SFC files (`.nemo`) that get
built and loaded."

The remaining gap is the entry file itself: `app.xml` is still a `<nemo>` XML
document parsed by `process_root()`, distinct from the SFC pipeline. Making it
an SFC unifies the authoring format — there is one language for both components
and the app — and enables control-flow directives (`n:for`/`n:if`) in the app
layout, which XML cannot express.

The build output is **not** `dist/app.xml`. The compiled output is an
implementation detail of the loader: the existing `dist/layout.json` (serialized
resolved `Value`) is a reasonable default; a binary format (bincode/postcard) is
a later optimization if profiling shows JSON parse dominates startup. The
output format is decided by the build system, not by this decision — the point
is that the *authoring* format is `.nemo` SFC, and the *output* is whatever the
loader reads efficiently.

# Consequences

* `app.nemo` is an SFC: it has `<template>` (the layout tree), optional
  `<props>`, `<style>`, `<script>`, and app-level blocks (`<app>`, `<data>`,
  `<imports>`, `<variable>`) that the compiler maps to the same `Value` tree
  keys `process_root` produces today. See the [`app.nemo` SFC entry
  plan](../plans/app-nemo-sfc-entry.md).
* The manifest `entry` default changes from `app.xml` to `app.nemo`
  (`crates/nemo-config/src/manifest.rs`). Existing projects with `entry =
  "app.xml"` keep working — the default is additive, not forced.
* `app.xml` remains supported as a legacy entry format (an explicit manifest
  `entry = "app.xml"` or a bare `--app-config app.xml` path), but new projects
  and templates default to `app.nemo`.
* The runtime's `load_config` accepts a `.nemo` entry by compiling it to the
  same `Value` tree the source path produces, then proceeding as today.
* The build output is the compiled `Value` tree (today's `dist/layout.json`),
  loaded via the existing `load_from_dist`. No `Value`→XML serializer is built.
* This decision does not change how component `.nemo` files work — they were
  already SFCs. It only extends the SFC format to the app entry.