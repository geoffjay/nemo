# Decisions

Records of significant decisions and their rationale.

* [XML is the configuration format (not HCL)](xml-not-hcl-config.md) - applications are configured in XML; HCL is not implemented.
* [Components implement RenderOnce, not Render](renderonce-for-components.md) - components are stateless and consumed on render.
* [The gpui git dependency is load-bearing in Cargo.lock](pin-gpui-git-dep.md) - a rev-less git dep means Cargo.lock pins the working revision; avoid `cargo update`.
* [Three-tier extension model with a unified PluginContext](three-tier-extensions.md) - Rhai, native cdylib, and WASM plugins share one host API.
* [cargo audit ignores transitive advisories we cannot upgrade](audit-ignore-transitive-advisories.md) - .cargo/audit.toml ignores 29 advisories pinned via the gpui/wasmtime deps; wasmtime upgrade tracked.
