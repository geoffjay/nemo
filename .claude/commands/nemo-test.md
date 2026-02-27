---
description: Run tests for Nemo crates — specific crate, all crates, or a test filter
---

# Run Nemo Tests

Run tests for the Nemo project.

## Arguments

The argument can be:
- A **crate name** (e.g., `config`, `data`, `layout`, `registry`, `extension`, `events`, `plugin-api`, `wasm`, `macros`, `nemo`)
- `all` — run all workspace tests
- A **test filter** (e.g., `test_hex_color`, `test_register_all`)
- No argument — run all tests

## Steps

### 1. Determine what to test

Map short names to full crate names:
| Short | Full Package |
|-------|-------------|
| `config` | `nemo-config` |
| `data` | `nemo-data` |
| `layout` | `nemo-layout` |
| `registry` | `nemo-registry` |
| `extension` | `nemo-extension` |
| `events` | `nemo-events` |
| `plugin-api` | `nemo-plugin-api` |
| `plugin` | `nemo-plugin` |
| `wasm` | `nemo-wasm` |
| `macros` | `nemo-macros` |
| `integration` | `nemo-integration` |
| `nemo` or `app` | `nemo` |

### 2. Run the tests

**Specific crate:**
```bash
cargo test -p <full-package-name>
```

**All crates:**
```bash
cargo test --workspace
```

**With test filter:**
```bash
cargo test --workspace -- <filter>
```

### 3. Report results

Show the test output. If tests fail, read the relevant source files to understand the failures and suggest fixes.

## Notes

- Some crates have integration tests in `tests/` directories
- The `nemo` crate has macro tests in `tests/nemo_component_macro.rs`
- WASM tests may require additional setup (wasmtime)
- Use `cargo test -p nemo-config -- --nocapture` to see println output
