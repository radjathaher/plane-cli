class PlaneCli < Formula
  desc "Plane CLI"
  homepage "https://github.com/radjathaher/plane-cli"
  version "0.1.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/radjathaher/plane-cli/releases/download/v0.1.1/plane-cli-0.1.1-darwin-aarch64.tar.gz"
      sha256 "2367013b17aeb17afed513dc1c4504113685c444786c510d83ec866d6a5a5205"
    else
      odie "plane-cli is only packaged for macOS arm64"
    end
  end

  def install
    bin.install "plane"
  end
end
