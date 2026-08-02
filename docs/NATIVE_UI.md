# Cosmic Scribe native UI — UX requirements

Last updated: 2026-08-02

Product: COSMIC desktop voice dictation. Native shell: **libcosmic**. Principle: **only what the user needs, in predictable places; nothing inert or unexplained.**

---

## Primary user stories

| ID | Story | Success |
|----|--------|---------|
| S1 | As a user who just dictated, I open History and see recent takes first | Newest at top; times readable (not all “just now”) |
| S2 | I open one take to read the full transcript | Clear open affordance; hover/press on row; full text in a bounded surface |
| S3 | I copy a transcript without opening detail | Copy control recognizable; toast “Copied” |
| S4 | I delete a bad take | Delete looks like trash; confirm before permanent delete |
| S5 | I load older takes | “Show more” only **after** the last loaded row (end of list content), not stuck on screen over latest items |
| S6 | I play audio and fix text | Play/Pause/Stop/seek; Edit → Save only if changed / Cancel; second play resets bar |
| S7 | I switch transcript source | Labels users understand (Transcript / Your edit / AI fix)—not “Original” alone |
| S8 | I connect SuperGrok | See **what auth STT uses now**; Sign in; plan/docs links; stored key status + remove |
| S9 | I change type vs clipboard | Preference persists; Save only when something is dirty |
| S10 | I scan History rows | Clear hover highlight; trash recognizable; preview = start + word/line stats |

---

## History list — must / must-not

| Must | Must not |
|------|----------|
| Rows use **visible hover** background (not invisible) | Hit targets with no highlight |
| Trailing chevron = “open this take” | Mystery open icons that look like export |
| Delete = **trash** icon (`edit-delete-symbolic`) + tooltip near cursor | Unrecognizable glyph without meaning |
| Copy = standard copy icon + tooltip near cursor | — |
| Preview = start of text + **word/line stats** (end ellipsis) | Mid-string `…` cuts that look broken |
| “Show more” **inside scroll**, **after** last list row, only if more pages exist | Sticky footer always visible while browsing latest items |
| Toast when more loaded / none left | Silent pagination |
| Empty state explains how to get recordings | Blank void |

**Show more placement rule:** Pagination control is the last child **inside** the scrollable list column (after `list_column` items). User scrolls past recent items to reach it—same as infinite lists on desktop.

---

## Recording detail — must / must-not

| Must | Must not |
|------|----------|
| Back: `go-previous-symbolic` + “History” | Misaligned “←” text-only |
| Page padding on all sides | Controls flush to window edge |
| Transcript in a **surface** (list/card container, padding, full width) | Floating body text mid-window with odd gaps |
| Version switcher only if ≥2 sources; labels from table below | Lone unexplained “Original” when only one version |
| Toolbar: Play, Stop (if loaded), Copy, Delete | Unrelated clutter |
| Edit: Save + Cancel; no new version if text unchanged | Save that clones identical text |

### Version / source labels (user-facing)

| Internal | Label shown | When |
|----------|-------------|------|
| base STT text (index 0) | **Transcript** | Always the speech recognition result |
| `user_edit` | **Your edit** (or Your edit 2…) | User saved an edit |
| `llm_correction` / other | **AI fix** (or AI fix 2…) | AI correction |

If only the base transcript exists, **hide** the version row entirely (no “Original” button).

---

## Settings — must / must-not

| Must | Must not |
|------|----------|
| Controls that change preferences | Always-on **Save** when nothing is dirty |
| Header Save **only when dirty** (disabled or hidden when clean) | Suggested Save that does nothing useful |
| Short descriptions on switch rows | Multi-paragraph walls beside segmented controls |
| Tray legend = colored **capsules** (idle / red / blue) | Random symbolic icons that don’t match tray |
| xAI status + sign-in + plan links | Inert chrome |

**Dirty rule:** `settings_dirty` true after user changes lang, api key field, output mode, or time mode vs last loaded/saved snapshot. After successful save or load, dirty = false. Header shows Save only if dirty (or disabled with no on_press when clean—prefer **omit** when clean for minimal chrome).

**Auto-apply option (allowed):** segmented mode changes may auto-save immediately; then no header Save for those. Text fields still need Save or blur-save. Chosen approach: **explicit Save only when dirty** for all preference fields (predictable).

---

## Button hierarchy (global)

| Role | Widget | Examples |
|------|--------|----------|
| Primary commit | `button::suggested` | Save (when dirty), Sign in, Transcribe |
| Secondary | `button::standard` | Play, Stop, Copy, Cancel, Refresh |
| Destructive | `button::destructive` | Delete / Confirm delete |
| External | `button::link` | Grok plans |
| Icon-only | `button::icon` + **tooltip** | Delete trash, Copy |

---

## Anti-patterns (do not reintroduce)

1. Sticky “Show more” outside the scroll, always in the viewport.
2. Unlabeled or unrecognizable delete control.
3. Always-active header Save with empty dirty state.
4. “Original” with no explanation and no alternate versions.
5. Transcript as uncontained text in whitespace.
6. No hover feedback on list rows.
