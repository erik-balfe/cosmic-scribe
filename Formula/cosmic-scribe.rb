class CosmicScribe < Formula
  desc "Cosmic Scribe — voice dictation for COSMIC desktop (Wayland)"
  homepage "https://github.com/erik-balfe/cosmic-scribe"
  license "MIT"
  # Builds from master tarball (no separate git clone). Repo must be public on GitHub.
  url "https://github.com/erik-balfe/cosmic-scribe/archive/refs/tags/v0.3.1.tar.gz"
  version "0.3.1"
  sha256 "e6ddf52754f620b0c7e8ff0128de6fe7bc5a5c953cbc56dc37a28e53218df7ca"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  def caveats
    on_linux do
      <<~EOS
        Install system dependencies (Fedora example):
          sudo dnf install alsa-utils wl-clipboard wtype libnotify

        First-time setup:
          cosmic-scribe --install
          # App window (History/Settings): clone repo and run scripts/install-gui-prod.sh
          # API key: open Cosmic Scribe → Settings

        Update later:
          brew upgrade cosmic-scribe
          "$(brew --prefix)/bin/cosmic-scribe" --update

        Uninstall user install + tray:
          "$(brew --prefix)/bin/cosmic-scribe" --uninstall
        Remove Homebrew package:
          brew uninstall cosmic-scribe

        Service: --start | --stop | --restart | --status

        Bind a global shortcut to: cosmic-scribe --trigger
        Tray: red capsule = recording, blue capsule = recognizing.
      EOS
    end
  end

  test do
    assert_match "cosmic-scribe", shell_output("#{bin}/cosmic-scribe 2>&1", 0)
  end
end