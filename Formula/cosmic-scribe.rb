class CosmicScribe < Formula
  desc "Cosmic Scribe — voice dictation for COSMIC desktop (Wayland)"
  homepage "https://github.com/erik-balfe/cosmic-scribe"
  license "MIT"
  # Builds from master tarball (no separate git clone). Repo must be public on GitHub.
  url "https://github.com/erik-balfe/cosmic-scribe/archive/refs/tags/v0.2.0.tar.gz"
  version "0.2.0"
  sha256 "1eb88cfeed8dd632e070ad9497ce63e48ccd01b037157ec9607d20c1d844d37a"

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
          cosmic-scribe --configure

        Update later:
          brew upgrade cosmic-scribe
          "$(brew --prefix)/bin/cosmic-scribe" --update

        Uninstall user install + tray:
          "$(brew --prefix)/bin/cosmic-scribe" --uninstall
        Remove Homebrew package:
          brew uninstall cosmic-scribe

        Service: --start | --stop | --restart | --status

        Bind a global shortcut to: cosmic-scribe --trigger
        Tray: mic icon → solid red dot while recording.
      EOS
    end
  end

  test do
    assert_match "cosmic-scribe", shell_output("#{bin}/cosmic-scribe 2>&1", 0)
  end
end