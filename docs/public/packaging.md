# Packaging & Distribution

How Nemo is built, packaged, and distributed. See `README.md` for the
end-user installation instructions.

## Release artifacts

The `Release` workflow (`.github/workflows/release.yml`) runs on `v*` tags (and
manual `workflow_dispatch`). It first gates on the full CI workflow, then builds
a matrix of targets and uploads to a GitHub Release:

| Target | Archive | Also |
|--------|---------|------|
| `aarch64-apple-darwin` | `nemo-aarch64-apple-darwin.tar.gz` | `Nemo-*.zip` + `Nemo-*.dmg` (`.app`) |
| `x86_64-apple-darwin` | `nemo-x86_64-apple-darwin.tar.gz` | `Nemo-*.zip` + `Nemo-*.dmg` (`.app`) |
| `x86_64-unknown-linux-gnu` | `nemo-x86_64-unknown-linux-gnu.tar.gz` | `.deb` + `.rpm` |
| `aarch64-unknown-linux-gnu` | `nemo-aarch64-unknown-linux-gnu.tar.gz` | `.deb` + `.rpm` |
| `x86_64-pc-windows-msvc` | `nemo-x86_64-pc-windows-msvc.zip` | — |

A `checksums.txt` (SHA-256 of every archive, `.deb`, `.rpm`, and `.dmg`) is
uploaded alongside.

The tarball layout is:

```
nemo               # the binary
share/nemo/        # bundled examples
```

## Distribution channels

### Install script

`scripts/install.sh` detects OS/arch, downloads the matching `tar.gz`, verifies
it against `checksums.txt`, and installs the binary. It is served from `main`, so
it always reflects the latest release. No crates.io publish is involved.

### Homebrew tap

The `nemo` binary cannot be published to crates.io (it depends on a git build of
GPUI), so distribution is via a binary tap (`geoffjay/homebrew-tap`) rather than
`brew install --HEAD`:

```bash
brew install geoffjay/tap/nemo
```

**Automated on release.** The `homebrew` job in `release.yml` renders the formula
from the release's `checksums.txt` and pushes it to `Formula/nemo.rb` in the tap.
It is gated on a `HOMEBREW_TAP_TOKEN` repository secret (a PAT with
`contents:write` on `geoffjay/homebrew-tap`); if the secret is absent the job
logs a notice and skips, so releases still succeed.

**Manual regeneration** (e.g. to seed or fix the formula):

```bash
# Renders to stdout from packaging/homebrew/nemo.rb.tpl using the
# checksums.txt attached to the v<version> release.
scripts/gen-homebrew-formula.sh v0.6.0 > Formula/nemo.rb
```

`packaging/homebrew/nemo.rb.tpl` is the source of truth; the generator only
fills in the version and the four `tar.gz` checksums. It never writes back over
the template, so it is safe to run repeatedly.

### Debian / RPM packages

`cargo-deb` metadata (`[package.metadata.deb]`) builds the `.deb`: binary at
`/usr/bin/nemo`, examples under `/usr/share/nemo/examples`, and the GPUI runtime
libraries as `Depends:` so `apt` resolves them. `cargo-generate-rpm` metadata
(`[package.metadata.generate-rpm]`) builds the `.rpm`, auto-detecting runtime
library requirements from the binary's ELF. Both are built per Linux target in
the release matrix.

### cargo-binstall

`[package.metadata.binstall]` in `crates/nemo/Cargo.toml` maps the crate to the
release archives. Because `nemo` is not on crates.io, use the git form:

```bash
cargo binstall --git https://github.com/geoffjay/nemo nemo
```

## Known gaps / future work

- **macOS signing & notarization.** Artifacts are unsigned; users must clear the
  quarantine flag (`xattr -dr com.apple.quarantine ...`). Notarization needs an
  Apple Developer certificate in secrets and a `codesign` + `notarytool` step.
- **Linux AppImage.** `.deb` and `.rpm` cover the two major package managers; an
  AppImage would additionally serve distros without either. Bundling GPUI's
  Vulkan/font libraries is the fiddly part and needs a clean-distro test.
- **Windows `.msi`.** Ships as a portable `.zip` only. A `cargo-wix` installer
  and Authenticode signing (needs a cert) remain.
