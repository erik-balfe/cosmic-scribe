# Outreach — Cosmic Scribe on COSMIC desktop

Product name: **Cosmic Scribe** (repo/binary rename to `cosmic-scribe` planned).

## Where COSMIC users discover apps

| Channel | What to do |
|---------|------------|
| [System76 — COSMIC Apps](https://system76.com/cosmic/apps) | Community showcase; ask System76 / Pop!_OS channels how to submit a “Made by you” entry |
| **COSMIC Store** (on desktop) | Ship **Flatpak** on Flathub — Store indexes Flathub + `.deb` ([pop-os/cosmic-store](https://github.com/pop-os/cosmic-store)) |
| [Flathub](https://flathub.org/) | Long-term; tray + `arecord` need careful permissions (not trivial) |
| [Pop!_OS Shop](https://system76.com/pop) / support articles | Hardware buyers looking for software |
| **r/pop_os**, **r/COSMICdesktop** (if active), Lobsters, Fediverse | Release post + screenshots + 10s GIF |
| [GitHub `pop-os/cosmic-epoch`](https://github.com/pop-os/cosmic-epoch) | Discussions/issues only if relevant; not an app directory |
| [cosmic-applets](https://github.com/pop-os/cosmic-applets) | For panel applets only — Cosmic Scribe is a tray **daemon**, not an applet |

There is no single “COSMIC app registry” like GNOME Circle. Practical path: **README + GitHub topics** (`cosmic-desktop`, `pop-os`, `dictation`, `wayland`) → **Reddit/Lobsters** → **Flathub** when ready → ask about **system76.com/cosmic/apps** listing.

## Message angles

- Built on COSMIC daily; tray shows recording state (red dot).
- Shortcut dictation into any app; history for replay/edit.
- Requires xAI API key; honest about cloud STT.

## Before posting

- [ ] Repo public, one clean commit, screenshots in README
- [ ] Short screen recording (tray → speak → text)
- [ ] Rename crate/binary to `cosmic-scribe` (optional but clearer)