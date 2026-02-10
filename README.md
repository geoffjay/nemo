[![CI][ci-badge]][ci-url]
[![Release][release-badge]][release-url]
[![codecov][codecov-badge]][codecov-url]
[![MIT licensed][mit-badge]][mit-url]
[![Apache licensed][apache-badge]][apache-url]

[ci-badge]: https://github.com/geoffjay/nemo/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/geoffjay/nemo/actions/workflows/ci.yml
[release-badge]: https://github.com/geoffjay/nemo/actions/workflows/release.yml/badge.svg
[release-url]: https://github.com/geoffjay/nemo/actions/workflows/release.yml
[codecov-badge]: https://codecov.io/gh/geoffjay/nemo/graph/badge.svg?token=knPW8TUmoJ
[codecov-url]: https://codecov.io/gh/geoffjay/nemo
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/geoffjay/nemo/blob/main/LICENSE-MIT
[apache-badge]: https://img.shields.io/badge/License-Apache_2.0-yellowgreen.svg
[apache-url]: https://github.com/geoffjay/nemo/blob/main/LICENSE-APACHE

# Nemo

> [!WARNING]
> Nemo is in a Beta state, it's safe to use but breaking changes are possible.

![Nemo][logo]

A configuration-driven desktop application framework. Define UI, data sources, and event handlers in HCL -- Nemo renders a native, GPU-accelerated application.

Built on [GPUI](https://gpui.rs).

## Quick Start

```bash
cargo build --release
nemo --config app.hcl
```

A minimal application:

```hcl
app {
  window {
    title = "Hello Nemo"
  }
  theme {
    name = "kanagawa"
    mode = "dark"
  }
}

layout {
  type = "stack"

  component "greeting" {
    type = "label"
    text = "Hello, World!"
  }
}
```

## Features

- **Declarative UI** -- Component trees defined in HCL configuration
- **Live data binding** -- Connect timer, HTTP, WebSocket, MQTT, Redis, and NATS sources to components
- **Scripted logic** -- Event handlers written in RHAI
- **Theming** -- Built-in themes (Kanagawa, Catppuccin, Tokyo Night, Gruvbox, Nord) with dark/light modes
- **Extensible** -- Native plugin support via dynamic libraries

## Examples

```bash
nemo --config examples/basic/app.hcl
nemo --config examples/calculator/app.hcl
nemo --config examples/components/app.hcl
nemo --config examples/data-binding/app.hcl
```

## Documentation

Full documentation is available at [geoffjay.github.io/nemo](https://geoffjay.github.io/nemo) or locally via `mkdocs serve`.

## License

MIT OR Apache-2.0

<!-- links -->

[logo]: docs/assets/nemo.png
