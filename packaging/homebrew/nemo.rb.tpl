# typed: false
# frozen_string_literal: true

# Homebrew formula for Nemo.
#
# This file is the source of truth for the formula published to the
# geoffjay/homebrew-nemo tap. It is regenerated for each release by
# scripts/gen-homebrew-formula.sh, which fills in the version and the
# per-target sha256 checksums from the release's checksums.txt.
#
# See docs/packaging.md for the tap setup and release workflow.
class Nemo < Formula
  desc "Configuration-driven, GPU-accelerated desktop application framework"
  homepage "https://github.com/geoffjay/nemo"
  version "0.0.0"
  license "MIT OR Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/geoffjay/nemo/releases/download/v#{version}/nemo-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_AARCH64_APPLE_DARWIN"
    end
    on_intel do
      url "https://github.com/geoffjay/nemo/releases/download/v#{version}/nemo-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/geoffjay/nemo/releases/download/v#{version}/nemo-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_AARCH64_UNKNOWN_LINUX_GNU"
    end
    on_intel do
      url "https://github.com/geoffjay/nemo/releases/download/v#{version}/nemo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_X86_64_UNKNOWN_LINUX_GNU"
    end
  end

  def install
    bin.install "nemo"
    pkgshare.install Dir["share/nemo/*"] if Dir.exist?("share/nemo")
  end

  test do
    assert_match "nemo", shell_output("#{bin}/nemo --version")
  end
end
