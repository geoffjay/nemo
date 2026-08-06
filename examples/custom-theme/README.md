# Custom Theme

Demonstrates **project-level custom themes** — defining your own theme instead of
only selecting one of the themes baked into nemo.

## What it shows

- **`<themes>`** registers one or more theme sets from external JSON files. The
  files (`themes/aurora.json` here) use the exact same schema as the shipped
  themes (`crates/nemo/src/theme/*.json`), so the easiest way to author one is to
  copy a shipped theme and edit its colors.
- **`<theme name="aurora" mode="dark">`** selects the project-defined theme by its
  set name, exactly as you would select a shipped theme.
- **`<extend>`** overrides individual colors on top of the selected base theme.
  Overrides always win — here the primary button color is changed to orange
  without touching the rest of the Aurora palette.

Project-defined themes also appear in the runtime Settings picker (Ctrl+P),
alongside the shipped themes.

## Run

```sh
nemo run examples/custom-theme/app.nemo
# or capture a screenshot (macOS):
nemo screenshot examples/custom-theme/app.nemo
```
