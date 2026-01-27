class PlaneCli < Formula
  desc "Plane CLI"
  homepage "https://github.com/radjathaher/plane-cli"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/radjathaher/plane-cli/releases/download/v0.1.0/plane-cli-0.1.0-darwin-aarch64.tar.gz"
      sha256 "e6853b14e626e2ea3910838c3f1b7755fe7595762d7d86111283b35eccee25cf"
    else
      odie "plane-cli is only packaged for macOS arm64"
    end
  end

  def install
    bin.install "plane"
  end
end
