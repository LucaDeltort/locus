class Locus < Formula
  desc "A fast, open-source tool for managing iOS/macOS localization files"
  homepage "https://github.com/LucaDeltort/locus"
  url "https://github.com/LucaDeltort/locus/releases/download/v0.1.0/locus-macos-universal.tar.gz"
  version "0.1.0"
  sha256 ""

  def install
    bin.install "locus"
  end

  test do
    assert_match "locus", shell_output("#{bin}/locus --version")
  end
end
