# CLI Reference

Nemo is invoked from the command line. This page documents all available options and commands.

## Usage

```
nemo [OPTIONS] [COMMAND]
```

When no command is given, Nemo runs the application: it launches the window (or the
project loader when no `--app-config` is provided), or runs headless/validation when
those flags are set.

## Commands

Run `nemo <command> --help` for per-command options.

| Command | Description |
|---------|-------------|
| `nemo new <name>` | Scaffold a new project from a template |
| `nemo dev` | Run an application with hot-reload on configuration changes |
| `nemo validate <file>` | Validate a configuration file and exit |

> **Rollout status:** `nemo dev` is available. `nemo new` and `nemo validate` are
> not yet implemented and exit non-zero until their workstreams land. The default
> (no-command) run path below is fully supported.

## Options

These options apply to the default run path (no command).

| Option | Short | Env | Description |
|--------|-------|-----|-------------|
| `--app-config <PATH>` | | `NEMO_APP_CONFIG` | Path to the main XML configuration file (`app.xml`) |
| `--config <PATH>` | `-c` | `NEMO_CONFIG` | Path to the TOML application config (`config.toml`) |
| `--app-config-dirs <DIR>` | `-d` | | Additional configuration directories to scan (repeatable) |
| `--extension-dirs <DIR>` | `-e` | `NEMO_EXTENSION_DIRS` | Extension/plugin directories, `:`-separated (repeatable) |
| `--verbose` | `-v` | | Enable debug-level logging (global; also works with commands) |
| `--watch` | | | Watch the config directory and hot-reload on changes (like `nemo dev`) |
| `--headless` | | | Run without opening a window |
| `--validate-only` | | | Deprecated — prefer `nemo validate`. Parse and validate config, then exit |
| `--help` | `-h` | | Print help information |
| `--version` | `-V` | | Print version |

## Examples

### Run an application

```bash
nemo --app-config app.xml
```

### Run with verbose logging

```bash
nemo --app-config app.xml --verbose
```

### Develop with hot-reload

```bash
nemo dev --app-config app.xml
```

Runs the app and reloads it automatically when `app.xml`, files under its
directory (including `.rhai` handlers), or extension directories change.
`--debounce-ms` tunes the settle window (default 200 ms). An invalid edit shows
an error and leaves the last working UI running. The same behavior is available
on the default run path via `--watch`:

```bash
nemo --app-config app.xml --watch
```

### Validate configuration without launching

```bash
nemo validate app.xml
```

This parses the XML file, checks for syntax errors and schema violations, then exits.
Useful in CI pipelines or before deploying configuration changes. The legacy
`nemo --app-config app.xml --validate-only` form remains supported.

### Run in headless mode

```bash
nemo --app-config app.xml --headless
```

Starts data sources and event handling without opening a window. Useful for background data processing or testing. Press `Ctrl-C` to stop.

### Load additional config and extension directories

```bash
nemo --app-config app.xml -d ./config.d -e ./plugins -e ./scripts
```

Multiple directories can be specified by repeating the flag. Config directories are scanned for additional XML files. Extension directories are scanned for `.rhai` scripts and native plugin libraries.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (including a valid `nemo validate` / `--validate-only`) |
| 1 | Configuration error (parse, validation, or resolution failure) |
| 1 | Runtime error (failed to initialize subsystems) |

## Environment

Nemo reads environment variables in XML expressions via the `${env.VARIABLE_NAME}` syntax. The options table above lists the environment variables that provide defaults for CLI flags (`NEMO_APP_CONFIG`, `NEMO_CONFIG`, `NEMO_EXTENSION_DIRS`).

## Logging

Nemo uses [tracing](https://docs.rs/tracing) for structured logging. Output goes to stderr.

- Default level: `INFO`
- With `--verbose`: `DEBUG`

Log output includes thread IDs and module targets for troubleshooting:

```
2026-02-09T12:00:00.000Z  INFO nemo: Nemo v0.1.0 starting...
2026-02-09T12:00:00.001Z  INFO nemo::runtime: Loading configuration from: "app.xml"
2026-02-09T12:00:00.010Z  INFO nemo::runtime: Initializing subsystems...
2026-02-09T12:00:00.015Z  INFO nemo: Starting GPUI application...
```
