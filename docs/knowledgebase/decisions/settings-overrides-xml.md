---
type: Decision
title: Project settings persist to an `overrides.xml` overlay (not the entry file)
description: Theme choices from the settings UI are written to `overrides.xml` next to the entry, not edited into `app.nemo`/`app.xml`. The runtime merges the overlay at load time. Keeps the source entry immutable regardless of format.
tags: [config, sfc, settings, decision]
timestamp: 2026-08-05T00:00:00Z
---

# Decision

Project-level settings (today: the theme `name` and `mode` from the settings UI)
are persisted to an **`overrides.xml`** overlay file sitting next to the entry
(`app.nemo` or `app.xml`), not edited into the entry file itself. The runtime
merges the overlay's `app` key into the loaded config at load time, so the
overlay takes precedence over the entry's `<theme>`.

# Context

Before the `.nemo` SFC entry, `xml_edit::set_app_theme` did surgical text edits
to `app.xml` to persist theme choices — updating the `<theme>` element's
`name`/`mode` attributes in place. This worked because `app.xml` is plain XML:
the text scanner (`find_opening_tag`) could safely locate `<theme>` and `<app>`.

With `app.nemo` as the entry, the text-edit approach is **fragile**: an SFC
carries raw-text `<style>`/`<script>` blocks whose bodies are captured verbatim
(not parsed as XML). A CSS selector like `app { … }` or a script comment
mentioning `theme` could make `find_opening_tag` match inside a raw-text region,
corrupting the file. The single-root `<template>` constraint and the
raw-text/ XML interleaving make a robust text editor for `.nemo` significantly
harder than for plain XML.

Three options were considered (see the [`app.nemo` SFC entry
plan](../plans/app-nemo-sfc-entry.md) Phase 6):

1. **`overrides.xml` overlay** — keep the entry immutable, write settings to a
   separate `overrides.xml` that's merged at load. Clean separation, works
   with any entry format. Already mentioned as a "future option" in
   [`configuration.md`](../concepts/configuration.md).
2. **Adapt `xml_edit` to `.nemo`** — extend the text editor to handle SFC
   structure (skip raw-text regions). Fragile and SFC-specific.
3. **Move project settings to `nemo.toml [settings]`** — simplest, but
   conflates build config with runtime prefs.

# Chosen: `overrides.xml` overlay

The overlay is a tiny XML file — it only needs `<nemo><app><theme name="…"
mode="…" /></app></nemo>`, not SFC structure. It's parsed by the existing
`load_xml_string` path (no new parser), and the merge is a shallow override of
`config["app"]` keys. This:

* Keeps `app.nemo` (and `app.xml`) **immutable** — the source file is the
  source of truth; user prefs live alongside it.
* Is **format-agnostic** — the same mechanism works for `.nemo` and `.xml`
  entries. `app.xml` projects benefit too: the settings UI no longer mutates
  the hand-authored file.
* Is **trivial to parse** — `overrides.xml` is plain XML, no raw-text blocks,
  no SFC single-root constraint.
* **Degrades gracefully** — if `overrides.xml` doesn't exist (the common
  case), the entry's `<theme>` is used as today. If it exists but is malformed,
  it's skipped with a warning.

# Consequences

* `xml_edit::set_app_theme(entry_path, name, mode)` now writes to
  `<entry_dir>/overrides.xml` (creating or updating it), not to the entry file.
  The pure `set_theme_in_xml` string transform is reused for the overlay
  content. The entry file is never mutated.
* `NemoRuntime::load_config` (`runtime.rs`), after loading the main config,
  checks for `overrides.xml` next to the config path and merges its `app` key
  (shallow merge: overlay `app` keys override the config's `app` keys). Only
  the runtime applies the overlay — `nemo build`, `nemo validate`, and `nemo
  schema` operate on the source entry, not the effective (overridden) config.
  This keeps `dist/layout.json` a faithful compile of the source.
* `apply_theme_from_runtime` (`workspace/utils.rs`) reads
  `runtime.get_config("app.theme.name")` — after the merge, this picks up the
  overridden theme, so the project layer still wins over the global layer.
* Existing `app.xml` projects: the settings UI now writes to `overrides.xml`
  instead of editing `app.xml`. A project with no `overrides.xml` behaves
  exactly as before (the entry's `<theme>` is used). A project with an existing
  edited `<theme>` in `app.xml` keeps it as the fallback; a new `overrides.xml`
  takes precedence once created.
* The overlay is not watched for hot-reload (it's a generated file, not a
  source file; editing it via the settings UI already triggers a reload via
  the existing settings-change path).