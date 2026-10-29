# Decisions

Records of significant decisions and their rationale.

* [XML is the configuration format (not HCL)](xml-not-hcl-config.md) - applications are configured in XML; HCL is not implemented.
* [Components implement RenderOnce, not Render](renderonce-for-components.md) - components are stateless and consumed on render.
* [GPUI comes from crates.io via the gpui-kit umbrella and the gpui-pre snapshots](gpui-kit-crates-io-dep.md) - gpui-kit pins the gpui-pre snapshot crates exactly; no git deps, Cargo.lock no longer load-bearing against git drift.
* [Three-tier extension model with a unified PluginContext](three-tier-extensions.md) - Rhai, native cdylib, and WASM plugins share one host API.
* [cargo audit ignores transitive advisories we cannot upgrade](audit-ignore-transitive-advisories.md) - .cargo/audit.toml ignores the advisories pinned via the gpui-kit (gpui-pre snapshot) and wasmtime 29 stacks; wasmtime upgrade tracked.
* [nemo screenshot uses gpui's test-support render-to-image path](screenshot-via-test-support-feature.md) - opt-in `screenshot` feature enables offscreen `Window::render_to_image`; macOS-first, additive to Cargo.lock.
* [Screenshots target macOS; Windows out of scope](screenshots-windows-out-of-scope.md) - `nemo screenshot` is macOS-first; Linux best-effort/deferred, Windows out of scope.
* [The application entry is a `.nemo` SFC (not `app.xml`)](app-nemo-sfc-entry.md) - `app.nemo` is an SFC compiled at build time; supersedes the XML entry decision. Build output is compiled (not `dist/app.xml`).
* [Project settings persist to an `overrides.xml` overlay](settings-overrides-xml.md) - the settings UI writes to `overrides.xml` next to the entry, not into `app.nemo`/`app.xml`; the runtime merges it at load time.
* [Control-flow directives use `n:for`/`n:if`; `n:for` evaluates over live data](control-flow-directives.md) - Vue-style namespaced attributes; `n:if` is compile-time, `n:for` over live data is a runtime list-binding expansion.
* [SFC authoring is XML-native; Rhai-style script ergonomics are deliberately deferred](sfc-script-ergonomics-deferred.md) - the Vue-style SFC concept is fully covered today (imports, props, slots, style, per-SFC scripts); `<script setup>` imports, `setup(props)`, and moustache interpolation are deferred.
