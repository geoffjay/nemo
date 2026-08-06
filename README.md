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

A configuration-driven desktop application framework. Define UI, data sources, and event handlers in XML -- Nemo renders a native, GPU-accelerated application.

Built on [GPUI](https://gpui.rs).

## Installation

### Install script (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/geoffjay/nemo/main/scripts/install.sh | sh
```

Installs the latest release binary to `~/.local/bin` (override with `NEMO_INSTALL_DIR`,
or pin a version with `NEMO_VERSION=v0.6.0`).

### Homebrew (macOS / Linux)

```bash
brew install geoffjay/tap/nemo
```

### Prebuilt binaries

Download the archive for your platform from the [latest release][release-latest]
and verify it against `checksums.txt`.

- **macOS:** the app and binary are **not code-signed or notarized**, so Gatekeeper
  blocks them on first launch. Remove the quarantine attribute after downloading:

  ```bash
  xattr -dr com.apple.quarantine /Applications/Nemo.app   # app bundle
  xattr -d  com.apple.quarantine ./nemo                   # CLI binary
  ```

- **Linux:** the `.tar.gz` binary is dynamically linked. Either install the `.deb`
  (which pulls in its dependencies automatically) or install the runtime libraries:

  ```bash
  sudo apt-get install -y libfontconfig1 libfreetype6 libvulkan1 \
    libxcb1 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0
  ```

### Build from source

```bash
cargo build --release
# binary at target/release/nemo
```

## Quick Start

```bash
nemo --app-config app.nemo
```

A minimal application (`app.nemo` — a single-file component entry):

```nemo
<app title="Hello Nemo">
  <window title="Hello Nemo" />
  <theme name="kanagawa" mode="dark" />
</app>

<template name="app">
  <stack id="root">
    <label id="greeting" text="Hello, World!" />
  </stack>
</template>
```

> The application entry is an `app.nemo` single-file component. Legacy `app.xml`
> entries are no longer supported — the loader rejects them; rename to `app.nemo`
> (remove the `<nemo>` wrapper, move the `<layout>` body into
> `<template name="app">`). XML remains valid only inside `<include>` fragments.

## Features

- **Declarative UI** -- Component trees defined in `.nemo` single-file components
- **Live data binding** -- Connect timer, HTTP, WebSocket, MQTT, Redis, and NATS sources to components
- **Scripted logic** -- Event handlers written in RHAI
- **Theming** -- Built-in themes (Kanagawa, Catppuccin, Tokyo Night, Gruvbox, Nord) with dark/light modes
- **Extensible** -- Native plugin support via dynamic libraries
```bash
nemo --app-config examples/basic/app.nemo
nemo --app-config examples/calculator/app.nemo
nemo --app-config examples/sfc/app.nemo
nemo --app-config examples/data-binding/app.nemo     # multi-file <include> example
nemo --app-config examples/components/app.nemo
```

Configuations are available in the [examples](examples) directory:

- [basic](examples/basic): A minimal application
- [calculator](examples/calculator): A calculator application
- [components](examples/components): A component-based application
- [data-binding](examples/data-binding): A data-binding application
- [data-streaming](examples/data-streaming): A data-streaming application

## Documentation

Full documentation is available at [geoffjay.github.io/nemo][docs] or locally via `zensical serve`.

Plugin API documentation is also available for [nemo-plugin][docs-nemo-plugin], [nemo-plugin-api][docs-nemo-plugin-api],
and [nemo-wasm-guest][docs-nemo-wasm-guest].

## License

MIT OR Apache-2.0

<!-- links -->

[logo]: docs/assets/nemo.png
[release-latest]: https://github.com/geoffjay/nemo/releases/latest
[docs]: https://geoffjay.github.io/nemo
[docs-nemo-plugin]: https://docs.rs/nemo-plugin
[docs-nemo-plugin-api]: https://docs.rs/nemo-plugin-api
[docs-nemo-wasm-guest]: https://docs.rs/nemo-wasm-guest
