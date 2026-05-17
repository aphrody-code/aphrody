# Homebrew formula for aphrody.
#
# Distribution: aphrody-code/tap (https://github.com/aphrody-code/homebrew-tap)
# Install: `brew install aphrody-code/tap/aphrody`
#
# Update on release: replace __VERSION__ + __SHA256_*__ placeholders. The
# .github/workflows/release.yml job emits per-target SHA-256 files alongside
# the archives; a follow-up release-please workflow PRs the bumped formula
# into the tap repo.

class Aphrody < Formula
  desc "Cross-platform Rust CLI (Linux / Windows / macOS / WebAssembly)"
  homepage "https://github.com/aphrody-code/aphrody"
  version "__VERSION__"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/aphrody-code/aphrody/releases/download/v#{version}/aphrody-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_DARWIN_ARM64__"
    end
    on_intel do
      url "https://github.com/aphrody-code/aphrody/releases/download/v#{version}/aphrody-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_DARWIN_X64__"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/aphrody-code/aphrody/releases/download/v#{version}/aphrody-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_ARM64__"
    end
    on_intel do
      url "https://github.com/aphrody-code/aphrody/releases/download/v#{version}/aphrody-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_LINUX_X64__"
    end
  end

  def install
    bin.install "aphrody"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/aphrody --version")
  end
end
