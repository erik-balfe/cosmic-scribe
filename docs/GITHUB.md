# GitHub

**Primary repo:** https://github.com/erik-balfe/cosmic-scribe (private)

**Legacy repo:** https://github.com/erik-balfe/voice-input — superseded by `cosmic-scribe`; kept until migration is confirmed.

## Remotes (jj)

```bash
cd /path/to/cosmic-scribe   # local dir may still be named voice-input
jj git remote list
# origin  → git@github.com:erik-balfe/cosmic-scribe.git
# voice-input → git@github.com:erik-balfe/voice-input.git  (optional, legacy)
```

Push to primary:

```bash
jj git push --bookmark master
```

## Going public

1. Complete [RELEASE.md](RELEASE.md)
2. `gh repo edit erik-balfe/cosmic-scribe --visibility public`
3. [DISTRIBUTION.md](DISTRIBUTION.md) — Homebrew tap (repo must be public for tarball install)
4. [OUTREACH.md](OUTREACH.md)