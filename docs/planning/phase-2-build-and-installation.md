# Phase 2: Build, Installation & Developer Experience — Implementation Plan

> **Plan Date:** 2026-07-10
> **Source:** `docs/nemo-improvement-roadmap.md` §2 ("Build and Installation")
> **Version at planning:** 0.6.0
> **Status:** Planning
> **Dependency on Phase 1 (Storybook):** None. Every workstream below is
> independent of the component-gallery work. A prior storybook attempt exists
> on the `feature/storybook` branch and can be evaluated separately.

---

## 1. Executive Summary

The May roadmap's §2 predates a round of packaging/distribution work that has
since landed. This plan re-baselines §2 against the **current** state of the
tree, then lays out an actionable path for the remaining items plus one
exploratory capability the roadmap did not cover: a **headless renderer for
screenshots**, to close the feedback loop for vision-capable AI models.

The two biggest corrections versus the roadmap:

- **The `futures` conflict (§2.1) is already resolved.** The workspace pins
  `futures = "0.3.32"` (`Cargo.toml:46`), `nemo-data` inherits it
  (`crates/nemo-data/Cargo.toml:12`), and `Cargo.lock` agrees. No action needed.
- **Hot-reload (§2.3) is far cheaper than estimated.** The roadmap assumed the
  layout-rebuild path did not exist. It does:
  `Workspace::reload_config` (`crates/nemo/src/workspace/mod.rs:150`) already
  tears down the old runtime and rebuilds the entire app from the config path,
  and is wired to `ctrl-shift-r` (`crates/nemo/src/main.rs:112`). The remaining
  work is only a file-watcher that dispatches the existing `ReloadConfig` action.

### Current state of each roadmap item

| Roadmap item | Status | Notes |
|--------------|--------|-------|
| §2.1 Fix `futures` conflict | ✅ Done | Resolved in the workspace; no longer reproduces. |
| §2.2 `nemo new` scaffold | ❌ Not started | Needs CLI subcommand support first. |
| §2.3 Hot-reload dev mode | 🟡 90% infra exists | Full rebuild path present; needs a watcher + trigger. |
| §2.4 Cross-platform packaging | 🟢 Mostly done | 5-target matrix, macOS `.app`, Linux `.deb`, Windows `.zip`. Gaps: `.dmg`, AppImage, `.rpm`, `.msi`, signing. |
| §2.5 Distribution | 🟢 Mostly done | `install.sh`, Homebrew formula + generator, binstall metadata, GitHub Releases + checksums. Gaps: create the tap repo, auto-push formula. |
| §2.6 `validate` subcommand | 🟡 Flag exists | `--validate-only` works (`args.rs:38`); promote to a subcommand with strict mode. |
| **NEW** Headless renderer / screenshots | ❌ Exploratory | Not in roadmap; see §9. |

---

## 2. Goals & Non-Goals

### Goals

- Make the **inner development loop** fast: edit config → see the result without
  a manual restart (`nemo dev`).
- Make **onboarding** a one-command experience (`nemo new`).
- Make **validation** a first-class, discoverable command with actionable output.
- Close the remaining **packaging/distribution gaps** so a fresh user on any of
  the three platforms can install and run without troubleshooting.
- Evaluate a **headless screenshot** capability so automated/AI workflows can
  *see* rendered output.

### Non-Goals

- The component storybook (Phase 1 §1) — explicitly out of scope here.
- The LLM authoring flow (Phase 4) — out of scope, though the screenshot work
  in §9 is a shared enabler.
- macOS/Windows **code signing** — tracked as a gap but blocked on certificates
  (no Apple Developer cert available at this time); the documented `xattr`
  workaround stands in for it.

---

## 3. Foundational Workstream A — CLI Subcommand Architecture

> **Status: ✅ Implemented (2026-07-10).** `Args` now carries an optional
> `command` (`crates/nemo/src/args.rs`); `main` dispatches to
> `crate::commands::{new,dev,validate}` with the default (no-subcommand) path
> preserved in `run_app`. `new`/`dev`/`validate` are wired with `--help` and
> honest not-yet-implemented stubs for Workstreams B/C/D. Covered by 8 unit
> tests (incl. the no-subcommand default guard); `cargo fmt`/`clippy` clean.

**Why first:** `nemo new`, `nemo dev`, `nemo validate` (and later `nemo storybook`,
`nemo screenshot`) all require subcommands. Today the CLI is a flat flag parser
(`crates/nemo/src/args.rs`, `#[derive(Parser)]` with no subcommands). This is the
enabling refactor for Workstreams B, D, and parts of C/G.

### Design

Introduce an optional subcommand while preserving 100% of today's behavior
(bare `nemo --app-config app.xml` must keep working, including the
project-loader screen when no config is given).

```rust
#[derive(Parser, Debug)]
#[command(name = "nemo", author, version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    // Existing global flags remain for the default (no-subcommand) run path.
    #[arg(long, env = "NEMO_APP_CONFIG")]
    pub app_config: Option<PathBuf>,
    // ... config, app_config_dirs, extension_dirs, verbose, headless, validate_only
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new Nemo project.
    New(NewArgs),
    /// Run an app with hot-reload on config changes.
    Dev(DevArgs),
    /// Validate a configuration file and exit.
    Validate(ValidateArgs),
    // Future: Storybook, Screenshot
}
```

`main()` dispatches on `args.command`: `None` → existing launch path (unchanged);
`Some(cmd)` → the corresponding handler. Keep `--validate-only` as a hidden alias
for one release cycle for backward compatibility, then deprecate.

### Tasks

1. Convert `Args` to hold `command: Option<Command>` + retain global flags.
2. Add a `commands/` module in the `nemo` crate (`commands/new.rs`,
   `commands/dev.rs`, `commands/validate.rs`) with a small `run(args) -> Result<()>`
   per command.
3. Route `main()` through the dispatcher; move the current launch body into a
   `commands::run_app` (or keep inline for `None`).
4. Update `docs/public/cli.md` and shell completion notes.

### Files
`crates/nemo/src/args.rs`, `crates/nemo/src/main.rs`, new `crates/nemo/src/commands/`.

### Acceptance
- `nemo --app-config app.xml` behaves exactly as before (incl. env vars).
- `nemo --help` lists `new`, `dev`, `validate`.
- `nemo <cmd> --help` shows per-command help.

**Effort:** Low–Medium · **Depends on:** nothing · **Risk:** Low (mechanical, but
touch every entry path; cover the no-subcommand default with a test).

---

## 4. Workstream B — `nemo new` Scaffold (§2.2)

**Objective:** `nemo new my-app [--template <t>]` generates a ready-to-run project.

### Design

- Templates: `basic`, `data-binding`, `calculator`, `complete` — sourced from the
  existing `examples/` directories (single source of truth). Prefer embedding
  them at build time via `include_dir!` so a standalone binary has no runtime
  path dependency on the repo.
- Output layout:
  ```
  my-app/
    app.xml
    scripts/handlers.rhai
    plugins/.gitkeep
    README.md
    .gitignore
  ```
- Refuse to overwrite a non-empty target dir unless `--force`.
- Print next steps (`cd my-app && nemo dev --app-config app.xml`).

### Tasks

1. Add `include_dir` (or `rust-embed`) dependency; embed `examples/<template>`.
2. Implement `commands/new.rs`: resolve template → copy embedded tree →
   substitute the project name into `app.xml`/`README.md` placeholders.
3. Add `--list` to enumerate templates.
4. Tests: scaffold each template into a temp dir; assert the result passes
   `nemo validate`.

### Files
`crates/nemo/src/commands/new.rs`, `crates/nemo/Cargo.toml` (embed dep),
templates drawn from `examples/`.

### Acceptance
- Each template scaffolds and immediately validates + runs.
- Existing-directory guard works; `--force` overrides.

**Effort:** Medium · **Depends on:** A · **Risk:** Low. Keep templates in sync
with `examples/` by generating from them, not copying by hand.

---

## 5. Workstream C — Hot-Reload Dev Mode (§2.3)

**Objective:** `nemo dev --app-config app.xml` runs the app and reloads on change.

### Current state (important)

The expensive part is already built:
`Workspace::reload_config` (`crates/nemo/src/workspace/mod.rs:150`) performs a
full rebuild — `shutdown` → `create_runtime` → rebuild header/footer/app entity →
re-apply theme → re-navigate → `window.refresh()`. It is dispatched today by the
`ReloadConfig` action (`ctrl-shift-r`). `notify = "6"` is already a workspace
dependency (used by `nemo-data`).

So hot-reload reduces to: **watch the right files and dispatch `ReloadConfig`
automatically, debounced.**

### Design

1. `nemo dev` starts the app as usual, then spawns a `notify` watcher on:
   - the primary `app_config` file,
   - any `<include>`d files (resolve from the parsed config),
   - the `scripts/` directory (`.rhai` handlers),
   - `app_config_dirs`.
2. Debounce events (~150–250 ms) to coalesce editor save storms.
3. On a settled change, marshal onto the GPUI foreground executor and dispatch
   `ReloadConfig` (or call `Workspace::reload_config` directly via an entity
   handle). The existing error-toast path handles invalid configs gracefully —
   the app stays up and shows the parse/validation error.
4. Watching runs on a background thread; bridge to the UI thread via a channel
   polled by a GPUI timer, or `cx.spawn` + async watcher stream.

### Tasks

1. Audit `reload_config` for completeness on include/script changes (does
   `create_runtime` re-read includes and re-load `.rhai`? — confirm and extend if
   not).
2. Implement `commands/dev.rs`: launch + watcher wiring + debounce.
3. Surface reload status in the footer/notification (already supported via
   `push_notification`).
4. Optional: expose the same behavior as a `--watch` flag on the default run path.

### Files
`crates/nemo/src/commands/dev.rs`, `crates/nemo/src/main.rs`,
`crates/nemo/src/workspace/mod.rs` (only if the reload audit finds gaps),
config-include resolution in `crates/nemo-config`.

### Acceptance
- Editing `app.xml` updates the running window within ~1 s, no restart.
- Editing an included file or a `.rhai` handler also reloads.
- An invalid edit shows an error toast and leaves the last-good UI running.

**Effort:** Low–Medium (was Medium in the roadmap; the rebuild path already
exists) · **Depends on:** A (for the subcommand; the `--watch` flag variant does
not) · **Risk:** Medium — the notify→UI-thread bridge and debounce need care;
watch include-path changes, not just the root file.

---

## 6. Workstream D — `nemo validate` Subcommand (§2.6)

**Objective:** promote the existing `--validate-only` flag to a discoverable
subcommand with actionable, non-zero-exit diagnostics and an optional strict mode.

### Design

- `nemo validate <app.xml> [--strict] [--format human|json]`.
- Reuse the runtime's existing load/validate path (`runtime.rs`
  `load_config`; `main.rs:69` already returns success on validation).
- Human format: `miette`-style diagnostics (already a workspace dep) with file,
  line, and a message. JSON format for editor/CI consumption.
- `--strict` additionally warns on: deprecated properties, components missing
  `id`, unused `<templates>`, and unknown attributes.
- Exit non-zero on error (today `--validate-only` returns `Ok(())`); wire real
  exit codes.

### Tasks

1. `commands/validate.rs` wrapping the runtime validate path.
2. Structured diagnostic type + human/JSON renderers.
3. Strict-mode lints (start with the four above; each is a registry/AST walk).
4. Keep `--validate-only` as a hidden alias that forwards to this.

### Files
`crates/nemo/src/commands/validate.rs`, `crates/nemo/src/args.rs`,
validation logic in `crates/nemo-config` / `crates/nemo-registry`.

### Acceptance
- `nemo validate good.xml` exits 0; `nemo validate bad.xml` exits 1 with a
  located error.
- `--format json` emits machine-readable diagnostics.
- `--strict` surfaces at least the four lint categories.

**Effort:** Low–Medium · **Depends on:** A · **Risk:** Low.

---

## 7. Workstream E — Packaging Gaps (§2.4)

**Already shipped:** 5-target release matrix, macOS `.app` bundle, Linux `.deb`
(`crates/nemo/Cargo.toml` `[package.metadata.deb]`), Windows portable `.zip`,
per-release `checksums.txt`, generated release notes with install + `xattr`
instructions (`.github/workflows/release.yml`).

### Remaining, in priority order

| Gap | Approach | Effort | Notes |
|-----|----------|--------|-------|
| macOS `.dmg` | `create-dmg` or `hdiutil` step in release matrix | Low | Nicer than a zipped `.app`. |
| Linux AppImage | `linuxdeploy` + `appimagetool` in CI | Medium | Covers non-Debian distros; bundles GPUI's runtime libs. Needs a `.desktop` + icon. |
| Linux `.rpm` | `cargo-generate-rpm` | Low–Medium | Fedora/RHEL parity with the `.deb`. |
| Windows `.msi` | `cargo-wix` | Medium | Proper installer vs. portable zip. |
| macOS notarization | `codesign` + `notarytool` | Blocked | Needs an Apple Developer cert. `xattr` workaround documented meanwhile. |
| Windows signing | Authenticode cert | Blocked | Needs a code-signing cert. |

### Tasks (unblocked subset)

1. Add `.dmg` packaging to the macOS matrix leg; upload + include in release.
2. Add an AppImage job for Linux (desktop file + icon under `assets/`,
   `linuxdeploy` with the GPUI libs, `appimagetool`).
3. Add `.rpm` via `cargo-generate-rpm` mirroring the `.deb` metadata.
4. Extend `checksums.txt` globbing and the release `files:` list to include the
   new artifacts.

### Files
`.github/workflows/release.yml`, `assets/` (`.desktop`, icon), possibly
`crates/nemo/Cargo.toml` (`[package.metadata.generate-rpm]`).

**Effort:** Medium overall · **Depends on:** nothing (parallel to A–D) ·
**Risk:** Medium — AppImage bundling of GPUI's Vulkan/font libs is fiddly; verify
the AppImage runs on a clean distro in CI.

---

## 8. Workstream F — Distribution Completion (§2.5)

**Already shipped:** `scripts/install.sh` (detect → download → checksum-verify →
install), Homebrew formula template + generator (`packaging/homebrew/nemo.rb.tpl`,
`scripts/gen-homebrew-formula.sh`), `[package.metadata.binstall]`, GitHub Releases.

### Remaining

1. **Create the `geoffjay/homebrew-nemo` tap repo** so `brew install
   geoffjay/nemo/nemo` resolves. Seed `Formula/nemo.rb` from the generator.
2. **Auto-push the formula on release.** Add a release job that runs
   `gen-homebrew-formula.sh` against the new `checksums.txt` and commits the
   result to the tap using a `HOMEBREW_TAP_TOKEN` secret (guarded so it no-ops
   when the secret is absent).
3. **Document the binstall limitation:** because `nemo` cannot be published to
   crates.io (git `gpui` dependency), `cargo binstall` requires the `--git` form;
   keep this called out in `docs/public/packaging.md`.
4. **Optional future channels:** AUR (`nemo-bin`), Scoop/Winget manifests.

### Files
External tap repo, `.github/workflows/release.yml` (tap-push job),
`docs/public/packaging.md`.

**Effort:** Low–Medium · **Depends on:** E is not required; benefits from a first
successful release · **Risk:** Low — cross-repo push needs the token + tap repo
to exist; gate it.

---

## 9. Workstream G (Exploratory) — Headless Renderer & Screenshots

> **User-requested; feasibility-gated.** The payoff: automated and AI-assisted
> workflows (e.g. the `/verify` and `/run` skills, Claude Code's vision) could
> capture what an app actually renders and *see* the result. Combined with
> hot-reload (§5), this closes an edit → render → observe loop for AI agents.

### The problem

GPUI renders through platform GPU backends (Metal on macOS, Blade/Vulkan on
Linux). `run_headless` (`crates/nemo/src/runtime.rs:404`) does **not** render — it
loads config and waits for a signal. There is no current path from "a running
Nemo app" to "a PNG of its window."

### Options (ranked by pragmatism)

**Option 1 — Real window + virtual display capture (recommended PoC).**
Launch the app normally under a headless display server, then screenshot it.
- **Linux:** run under `Xvfb` (or a headless Wayland compositor) with Mesa's
  software Vulkan (`lavapipe`, via `VK_ICD_FILENAMES`), so no physical GPU is
  needed. Capture the framebuffer with `ffmpeg -f x11grab`, `grim` (Wayland), or
  `import`. **This is the most CI-automatable path and needs no GPUI changes.**
- **macOS:** launch the app in a logged-in GUI session and use `screencapture
  -l<windowid>` (window id via CoreGraphics). Works locally; harder in headless CI.
- **Risk:** does GPUI/Blade initialize under `lavapipe`? Font rasterization and
  timing (wait for first paint + a settle delay) need tuning. **Validate this
  first — it de-risks the whole workstream.**

**Option 2 — True offscreen render-to-texture (ideal end state).**
Add a headless render path in GPUI: create a GPU surface backed by an offscreen
texture instead of a window swapchain, render one frame, read the pixels back,
encode PNG. Because we already patch `gpui` as a git dependency
(`Cargo.toml:1-2`), a fork/branch could expose this.
- **Pros:** deterministic, no display server, fast, CI-native, exact pixel size.
- **Cons:** requires GPUI internals work (or upstreaming); highest effort and the
  most uncertain — GPUI may not expose an offscreen surface publicly.

**Option 3 — Scene serialization + external renderer.** Rejected: duplicates the
renderer; too much surface area.

### Proposed shape

A `nemo screenshot --app-config app.xml --out out.png [--size WxH] [--settle-ms 500]`
subcommand that: launches, waits for first paint + settle, captures via the
platform mechanism (Option 1 initially), writes PNG, exits non-zero on failure.
Later, swap the capture backend to Option 2 without changing the CLI surface.

### Tasks (spike-first)

1. **Spike (time-boxed):** get *any* Nemo window to render under `Xvfb` +
   `lavapipe` on Linux and capture a non-blank PNG. Success/failure here decides
   whether Option 1 is viable. **This gate comes before any command design.**
2. If the spike passes: implement `commands/screenshot.rs` wrapping launch +
   settle + capture; add a CI job that screenshots each `examples/*` app and
   uploads the images as artifacts.
3. If deterministic/high-fidelity output is needed later: prototype Option 2 on a
   `gpui` fork branch (offscreen surface + readback).

### Files
`crates/nemo/src/commands/screenshot.rs`, a CI workflow (e.g.
`.github/workflows/screenshots.yml`), possibly a `gpui` fork branch for Option 2.

**Effort:** Spike Low; Option 1 Medium; Option 2 High · **Depends on:** A (for the
subcommand) · **Risk:** High and uncertain — **do the spike before committing.**
Recommendation: run the spike, then decide; do not schedule Option 2 until Option
1's fidelity is judged insufficient.

---

## 10. Sequencing & Dependencies

```
A (CLI subcommands)  ──┬──▶ B (nemo new)
                       ├──▶ D (nemo validate)
                       ├──▶ C (nemo dev)         [--watch flag variant needs no A]
                       └──▶ G (nemo screenshot)  [spike needs no A]

E (packaging gaps)   ── independent, parallelizable
F (distribution)     ── independent; benefits from one successful release
```

**Recommended order:**

1. **A** — small, unblocks the rest.
2. **C** — highest daily-value, and cheap now that the rebuild path exists.
3. **B** — onboarding.
4. **D** — validation UX.
5. **E** / **F** — in parallel with the above (different files; CI + packaging).
6. **G spike** — run early and independently to inform whether to invest; full
   implementation only if the spike passes.

---

## 11. Risks & Open Questions

- **A touches every entry path.** Guard the no-subcommand default with a test so
  the refactor can't silently break `nemo --app-config`.
- **C's reload scope.** Confirm `create_runtime` re-reads `<include>`s and
  reloads `.rhai` on rebuild; if not, extend before wiring the watcher.
- **E's AppImage** must be tested on a clean distro — bundling GPUI's Vulkan/font
  libraries is the failure-prone part.
- **F's tap** requires an external repo and a token; keep the auto-push job
  gated so releases succeed without it.
- **G is genuinely uncertain.** The single most valuable next action is the
  Xvfb + `lavapipe` spike; treat everything past it as conditional.
- **Signing (E)** stays blocked on certificates; the `xattr` workaround is the
  documented interim and should remain in the release notes until then.

---

## 12. Appendix — Key Files

| Area | Path |
|------|------|
| CLI args | `crates/nemo/src/args.rs` |
| Entry / dispatch | `crates/nemo/src/main.rs` |
| Full-rebuild reload | `crates/nemo/src/workspace/mod.rs:150` (`reload_config`) |
| Reload action | `crates/nemo/src/workspace/actions.rs`, `main.rs:112` |
| Runtime load/validate/headless | `crates/nemo/src/runtime.rs` (`load_config`, `run_headless:404`) |
| Script reload | `crates/nemo-extension/src/lib.rs:148`, `rhai_engine.rs:178` |
| Release CI | `.github/workflows/release.yml` |
| CI (reusable) | `.github/workflows/ci.yml` |
| Install script | `scripts/install.sh` |
| Homebrew | `packaging/homebrew/nemo.rb.tpl`, `scripts/gen-homebrew-formula.sh` |
| Packaging metadata | `crates/nemo/Cargo.toml` (`[package.metadata.deb]`, `[package.metadata.binstall]`) |
| Examples (scaffold sources) | `examples/` |
| Prior storybook attempt | branch `feature/storybook` |
