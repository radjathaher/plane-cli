class PlaneCli < Formula
  desc "Plane CLI"
  homepage "https://github.com/radjathaher/plane-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/radjathaher/plane-cli/releases/download/v0.1.0/plane-cli-0.1.0-darwin-aarch64.tar.gz"
      sha256 "cb09dbe73cf67fa5bf1ce1f208fe8cdb22cee88c8bc3d67840f19260a037fdd4"
    else
      odie "plane-cli is only packaged for macOS arm64"
    end
  end

  def install
    bin.install "plane"
  end
end
