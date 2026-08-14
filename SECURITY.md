# Security

## Reporting a vulnerability

Email **erik.balfe@proton.me** with a description and steps to reproduce. Do not open a public issue for undisclosed security problems.

## Scope

- API key storage (`~/.local/share/cosmic-scribe/`)
- Local recording files under `~/.local/share/cosmic-scribe/recordings/`
- Network traffic to the configured STT endpoint (default xAI)
- Opt-in usage numbers: off by default; no transcript, audio, or account id. Remote POST only if opted in **and** `COSMIC_SCRIBE_TELEMETRY_URL` (https only) is set.

## Out of scope

- xAI or OpenRouter service availability and policy
- Compromise of your own API keys via unrelated malware