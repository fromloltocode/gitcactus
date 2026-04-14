# typed: false
# frozen_string_literal: true

# Homebrew formula for GitCactus.
#
# This is intended for the custom tap at
#   https://github.com/fromloltocode/homebrew-tap
#
# After tagging a new release on GitHub:
#   1. Update the `url` to point at the tagged source tarball.
#   2. Update `sha256` with the output of:
#        curl -L <tarball-url> | shasum -a 256
#   3. Bump the `version` line if you pin it explicitly.
#   4. Commit and push to the tap repo.
#
# See docs/releasing.md for the full checklist.
class Gitcactus < Formula
  desc "Retro-inspired terminal Git assistant with beginner-friendly UX"
  homepage "https://github.com/fromloltocode/gitcactus"
  url "https://github.com/fromloltocode/gitcactus/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/fromloltocode/gitcactus.git", branch: "main"

  depends_on "rust" => :build

  def install
    # `std_cargo_args` installs the `gitcactus` binary into #{bin}.
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    # Basic sanity check: --version should print the package name and
    # a semver-shaped version string.
    output = shell_output("#{bin}/gitcactus --version")
    assert_match(/gitcactus \d+\.\d+\.\d+/, output)

    # --help should mention the binary name and the --version flag.
    help = shell_output("#{bin}/gitcactus --help")
    assert_match "gitcactus", help
    assert_match "--version", help
  end
end
