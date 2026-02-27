---
description: Validate a Nemo XML configuration file against component schemas and data source types
---

# Validate Nemo Configuration

Validate a Nemo XML configuration file for correctness without running the application.

## Arguments

The argument should be a path to an XML config file (e.g., `examples/basic/app.xml`).

## Steps

### 1. Read the XML file

Read the specified XML configuration file.

### 2. Validate structure

Check the top-level structure:
- Root element must be `<nemo>`
- Valid top-level children: `<app>`, `<variable>`, `<script>`, `<template>`, `<data>`, `<layout>`, `<plugin>`

### 3. Validate `<app>` section

- `<window>` — optional attributes: `title`, `width`, `height`, `min-width`, `min-height`
- `<header-bar>` — optional attributes: `github-url`, `theme-toggle`
- `<theme>` — `name` (kanagawa, tokyo-night, nord), `mode` (dark, light)

### 4. Validate `<data>` section

For each `<source>`, check:
- `name` attribute is present
- `type` is one of: timer, http, websocket, mqtt, redis, nats, file
- Required properties for each type are present (refer to the data source schemas in `crates/nemo-registry/src/builtins.rs`)

### 5. Validate `<layout>` section

For each component element, check by reading the component registry:
- The component type exists in the registry (read `crates/nemo-registry/src/builtins.rs`)
- Required properties are present
- Property types match expectations (e.g., boolean values for `disabled`, numeric for `width`)
- `on-click`/`on-change` handlers reference function names (should exist in scripts)
- `<binding>` children have `source` and `target` attributes

### 6. Validate expressions

Check `${...}` expressions:
- `${var.name}` — verify a matching `<variable name="...">` exists
- `${env.KEY}` — note that this depends on runtime environment
- Built-in functions: `upper()`, `lower()`, `trim()`, `length()`, `coalesce()`, `env()`

### 7. Validate scripts

If `<script src="...">` is present:
- Check that the directory exists
- Check for `.rhai` files
- Cross-reference handler names from `on-click`/`on-change` attributes with function definitions in Rhai scripts

### 8. Report results

Present findings organized as:
- **Errors** — things that will definitely fail (missing required props, unknown component types)
- **Warnings** — potential issues (unused variables, handler functions not found in scripts)
- **Info** — suggestions for improvement

## Component Types Reference

Read `crates/nemo-registry/src/builtins.rs` for the complete list of component types and their schemas. Key categories:

**Layout:** dock, stack, panel, tabs
**Display:** accordion, alert, avatar, badge, collapsible, dropdown_button, spinner, tag, label, icon, image, text, progress
**Input:** button, input, textarea, code_editor, text_editor, checkbox, select, radio, slider, switch, toggle
**Data:** table, list, tree
**Feedback:** modal, notification, tooltip
**Navigation:** sidenav_bar, sidenav_bar_item
**Charts:** line_chart, bar_chart, area_chart, pie_chart, candlestick_chart, column_chart, stacked_column_chart, clustered_column_chart, stacked_bar_chart, clustered_bar_chart, scatter_chart, bubble_chart, heatmap_chart, radar_chart, pyramid_chart, funnel_chart, realtime_chart

Note: XML uses kebab-case (`line-chart`), internally converted to snake_case (`line_chart`).
