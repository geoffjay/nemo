# Dev Dashboard

A developer-tools dashboard that demonstrates **every Rhai extension package**
available to nemo scripts in a single, cohesive application.

## Running

This example runs subprocesses (`cmd(...)`), which needs the `pkg-process`
Cargo feature — that one is **not** in the default build (a stock binary can't
spawn processes). The `rhai-env` and `rhai-sci` packages it also uses are on by
default, so `pkg-process` (or `all-packages`) is all you need:

```bash
cargo run --features pkg-process -- --app-config examples/dev-dashboard/app.nemo
```

Without `pkg-process` the app still launches, but the handlers log a warning
(`cmd(...) subprocess execution is unavailable`) and the System Info panel stays
empty.

The window opens maximized using the **nord** dark theme.

## What it demonstrates

| Panel | Package | What it does |
|-------|---------|-------------|
| **Clock** | `rhai-chrono` | `datetime_local().format("%H:%M:%S")` — live timestamp in the header |
| **Environment Variables** | `rhai-env` | `env("HOME")`, `env("SHELL")`, `env("USER")`, `env("PATH")` — reads process environment variables |
| **System Info** | `rhai-process` | `cmd(["uname", "-s"]).build().run()`, `cmd(["hostname"]).build().run()`, `cmd(["df", "-h"]).build().run()` — runs subprocesses and captures stdout |
| **HTTP Fetch** | built-in `http_get` | Fetches `https://httpbin.org/uuid` and parses the JSON response |
| **Response-Time Stats** | `rhai-sci` | `mean()`, `std()`, `min()`, `max()`, `median()` — computes statistics over HTTP response-time samples |

## How it works

The app XML opts in to the extension packages via the `features` attribute:

```xml
<script src="./scripts" features="file-io, system, science" />
```

- `system` enables `rhai-env` (environment variables) and `rhai-process`
  (subprocess execution).
- `science` enables `rhai-sci` (scientific computing / statistics).
- `file-io` is included for completeness (the dashboard doesn't persist to
  disk, but it's part of the feature set).

The **Take Sample** button makes a real HTTP request to `httpbin.org/delay/1`,
measures the round-trip time with `rhai-chrono` timestamps, and accumulates
statistics via `rhai-sci`'s `mean`/`std`/`min`/`max`/`median` functions.

## Security

This example enables `system` (subprocess spawning + environment access), which
grants scripts the ability to run arbitrary commands. Only run apps whose
scripts you trust. The default (no `features` attribute) preserves the sandbox.