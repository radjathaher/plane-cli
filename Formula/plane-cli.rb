class PlaneCli < Formula
  desc "Plane CLI"
  homepage "https://github.com/radjathaher/plane-cli"
  version "0.1.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/radjathaher/plane-cli/releases/download/v0.1.2/plane-cli-0.1.2-darwin-aarch64.tar.gz"
      sha256 "50c55d754178ca7da18723727028ac7aadfbfd23b7f737b33fbdd22a06b33501"
    else
      odie "plane-cli is only packaged for macOS arm64"
    end
  end

  def install
    bin.install "plane"
  end
end
