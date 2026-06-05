class VoiceInput < Formula
  desc "Cosmic Scribe — voice dictation for COSMIC desktop (Wayland)"
  homepage "https://github.com/erik-balfe/cosmic-scribe"
  license "MIT"
  # Builds from master tarball (no separate git clone). Repo must be public on GitHub.
  url "https://github.com/erik-balfe/cosmic-scribe/archive/refs/tags/v0.1.0.tar.gz"
  version "0.1.0"
  sha256 "8daa62e3c36e8257167aa8a9858ad41b00be7f6556b208af89814fda84767817"

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
          voice-input --install
          voice-input --configure

        Update later:
          brew upgrade voice-input
          "$(brew --prefix)/bin/voice-input" --update

        Uninstall user install + tray:
          "$(brew --prefix)/bin/voice-input" --uninstall
        Remove Homebrew package:
          brew uninstall voice-input

        Service: --start | --stop | --restart | --status

        Bind a global shortcut to: voice-input --trigger
        Tray: mic icon → solid red dot while recording.
      EOS
    end
  end

  test do
    assert_match "voice-input", shell_output("#{bin}/voice-input 2>&1", 0)
  end
end