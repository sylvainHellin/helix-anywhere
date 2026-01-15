# Homebrew formula for helix-anywhere
# To use this formula:
# 1. Create a GitHub release with the compiled binary
# 2. Update the url and sha256 below
# 3. Create a repo: sylvainhellin/homebrew-helix-anywhere
# 4. Copy this file to Formula/helix-anywhere.rb in that repo
#
# Users can then install with:
#   brew tap sylvainhellin/helix-anywhere
#   brew install helix-anywhere

class HelixAnywhere < Formula
  desc "Edit text from any application using Helix editor"
  homepage "https://github.com/sylvainhellin/helix-anywhere"
  version "0.1.0"
  license "MIT"

  # TODO: Update these URLs after creating a release
  on_macos do
    on_arm do
      url "https://github.com/sylvainhellin/helix-anywhere/releases/download/v0.1.0/helix-anywhere-darwin-arm64.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_ARM64"
    end
    on_intel do
      url "https://github.com/sylvainhellin/helix-anywhere/releases/download/v0.1.0/helix-anywhere-darwin-x86_64.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FOR_X86_64"
    end
  end

  depends_on :macos

  def install
    bin.install "helix-anywhere"
  end

  def caveats
    <<~EOS
      helix-anywhere requires Accessibility permissions to work.

      On first run, macOS will prompt you to grant permissions.
      You can also enable them manually in:
        System Settings > Privacy & Security > Accessibility > helix-anywhere

      Make sure you have Helix editor installed:
        brew install helix
    EOS
  end

  test do
    assert_match "helix-anywhere", shell_output("#{bin}/helix-anywhere --help 2>&1", 1)
  end
end
