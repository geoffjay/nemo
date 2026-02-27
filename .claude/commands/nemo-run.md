---
description: Run a Nemo example application by name or config path
---

# Run Nemo Example

Run a Nemo example application.

## Arguments

The argument can be:
- An **example name** (e.g., `basic`, `calculator`, `data-binding`, `components`, `data-streaming`, `pid-control`, `complete`)
- A **config file path** (e.g., `./my-app/app.xml`)

## Steps

### 1. Determine the config path

If the argument is an example name, resolve it to `examples/<name>/app.xml`.
If it's a file path, use it directly.

If no argument is provided, list available examples:
```bash
ls examples/*/app.xml
```
Then ask the user which one to run.

### 2. Verify the config exists

Check that the resolved config file exists. If not, inform the user.

### 3. Run the application

```bash
cargo run -- --config <config_path>
```

Run this command and let the user see the output. The application will open a GPUI window.

## Available Examples

| Name | Description |
|------|-------------|
| `basic` | Minimal app with a button |
| `calculator` | Interactive calculator with state management |
| `components` | Full component showcase |
| `data-binding` | Data sources, bindings, and transforms |
| `data-streaming` | Live NATS streaming with charts |
| `pid-control` | PID controller with template usage |
| `complete` | Complete feature demonstration |
