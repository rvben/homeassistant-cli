class HomeassistantCli < Formula
  desc "Agent-friendly Home Assistant CLI with JSON output, structured exit codes, and schema introspection"
  homepage "https://github.com/rvben/homeassistant-cli"
  version "0.1.4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/rvben/homeassistant-cli/releases/download/v0.1.4/homeassistant-cli-v0.1.4-aarch64-apple-darwin.tar.gz"
      sha256 "30ada29c2cffa223a1be7eab2f975bd6501f368eeb66ccd04e96e79a28fb290c"
    end
    on_intel do
      url "https://github.com/rvben/homeassistant-cli/releases/download/v0.1.4/homeassistant-cli-v0.1.4-x86_64-apple-darwin.tar.gz"
      sha256 "42a7b5becb6a10da4ad17e6ef2f98e4e44bb3fa93815bc952d2204a5ac0325cc"
    end
  end

  def install
    bin.install "ha"
  end

  def caveats
    <<~EOS
      Run `ha init` to set up your Home Assistant credentials.
      Run `ha config show` to verify your configuration.
    EOS
  end

  test do
    assert_match "ha #{version}", shell_output("#{bin}/ha --version")
  end
end
