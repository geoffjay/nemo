# CLI Reference

Nemo is invoked from the command line. This page documents all available options.

## Usage

```
nemo [OPTIONS]
```

## Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--config <PATH>` | `-c` | `app.hcl` | Path to the main HCL configuration file |
| `--config-dirs <DIR>` | `-d` | | Additional configuration directories to scan (repeatable) |
| `--extension-dirs <DIR>` | `-e` | | Extension/plugin directories to load (repeatable) |
| `--verbose` | `-v` | | Enable debug-level logging |
| `--headless` | | | Run without opening a window |
| `--validate-only` | | | Parse and validate config, then exit |
| `--help` | `-h` | | Print help information |
| `--version` | `-V` | | Print version |

## Examples

### Run an application

```bash
nemo --config app.hcl
```

### Run with verbose logging

```bash
nemo -c app.hcl --verbose
```

### Validate configuration without launching

```bash
nemo -c app.hcl --validate-only
```

This parses the HCL file, checks for syntax errors and schema violations, then exits. Useful in CI pipelines or before deploying configuration changes.

### Run in headless mode

```bash
nemo -c app.hcl --headless
```

Starts data sources and event handling without opening a window. Useful for background data processing or testing. Press `Ctrl-C` to stop.

### Load additional config and extension directories

```bash
nemo -c app.hcl -d ./config.d -e ./plugins -e ./scripts
```

Multiple directories can be specified by repeating the flag. Config directories are scanned for additional HCL files. Extension directories are scanned for `.rhai` scripts and native plugin libraries.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (including `--validate-only` with valid config) |
| 1 | Configuration error (parse, validation, or resolution failure) |
| 1 | Runtime error (failed to initialize subsystems) |

## Environment

Nemo reads environment variables in HCL expressions via the `${env.VARIABLE_NAME}` syntax. No special environment variables are required by Nemo itself.

## Logging

Nemo uses [tracing](https://docs.rs/tracing) for structured logging. Output goes to stderr.

- Default level: `INFO`
- With `--verbose`: `DEBUG`

Log output includes thread IDs and module targets for troubleshooting:

```
2026-02-09T12:00:00.000Z  INFO nemo: Nemo v0.1.0 starting...
2026-02-09T12:00:00.001Z  INFO nemo::runtime: Loading configuration from: "app.hcl"
2026-02-09T12:00:00.010Z  INFO nemo::runtime: Initializing subsystems...
2026-02-09T12:00:00.015Z  INFO nemo: Starting GPUI application...
```
