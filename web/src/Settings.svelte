<script>
  import Select from './lib/Select.svelte';

  let { historyTimeMode = $bindable('relative'), onsaved } = $props();

  let lang = $state('en');
  let outputMode = $state('wtype');
  let sttEndpoint = $state('https://api.x.ai/v1/stt');
  let hasKey = $state(false);
  let authMode = $state('none');
  let saved = $state(false);
  let error = $state('');
  let saving = $state(false);
  let apiKey = $state('');
  let analyticsOptIn = $state(false);
  let analyticsSummary = $state('');

  const outputOptions = [
    { value: 'wtype', label: 'Type into focus — inserts into the focused field (default)' },
    { value: 'clipboard', label: 'Clipboard only — copy for you to paste (terminals)' },
  ];

  const timeModeOptions = [
    { value: 'relative', label: 'Relative — e.g. 2h ago (hover for exact time)' },
    { value: 'absolute', label: 'Absolute — e.g. Jun 6, 9:15 PM (hover for relative)' },
  ];

  async function load() {
    error = '';
    try {
      const cr = await fetch('/api/config');
      if (!cr.ok) {
        error = `Failed to load settings (${cr.status})`;
        return;
      }
      const c = await cr.json();
      lang = c.lang || 'en';
      outputMode = c.output_mode === 'clipboard' ? 'clipboard' : 'wtype';
      historyTimeMode = c.history_time_mode === 'absolute' ? 'absolute' : 'relative';
      sttEndpoint = c.stt_endpoint || 'https://api.x.ai/v1/stt';
      hasKey = c.has_key;
      authMode = c.auth_mode || (c.has_key ? 'api_key' : 'none');
      analyticsOptIn = !!c.analytics_opt_in;
      analyticsSummary = c.analytics_summary || '';
    } catch {
      error = 'Failed to load settings';
    }
  }
  load();

  async function save(e) {
    e.preventDefault();
    saving = true;
    error = '';
    saved = false;
    const body = {
      lang,
      output_mode: outputMode,
      history_time_mode: historyTimeMode,
      stt_endpoint: sttEndpoint,
      analytics_opt_in: analyticsOptIn,
    };
    if (apiKey) body.key = apiKey;
    try {
      const r = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const text = await r.text();
      if (r.ok) {
        saved = true;
        apiKey = '';
        if (body.key) hasKey = true;
        setTimeout(() => saved = false, 2000);
        onsaved?.();
      } else {
        let msg = text.trim();
        try {
          const j = JSON.parse(text);
          msg = j.error || j.text || msg;
        } catch { /* plain text */ }
        error = msg || `Save failed (${r.status})`;
      }
    } catch {
      error = 'Network error — could not reach the app server';
    }
    saving = false;
  }
</script>

<form class="panel" onsubmit={save}>
  <section class="tray-legend" aria-label="Tray microphone states">
    <span class="legend-heading">Tray mic</span>
    <p class="field-hint legend-intro">The panel icon shows what Cosmic Scribe is doing. Left-click to record when idle.</p>
    <div class="legend-grid">
      <div class="legend-item">
        <svg class="mini-mic" viewBox="0 0 32 36" aria-hidden="true">
          <rect class="capsule idle" x="10" y="1" width="12" height="17" rx="6" />
          <path class="stand" d="M 9 18.5 Q 9 25.5 16 25.5 Q 23 25.5 23 18.5" />
          <line class="stand" x1="16" y1="25.5" x2="16" y2="29.5" />
          <line class="stand" x1="10" y1="33" x2="22" y2="33" />
        </svg>
        <div class="legend-text">
          <strong>Idle</strong>
          <span class="field-hint">Theme capsule — ready to record</span>
        </div>
      </div>
      <div class="legend-item">
        <svg class="mini-mic" viewBox="0 0 32 36" aria-hidden="true">
          <rect class="capsule recording" x="10" y="1" width="12" height="17" rx="6" />
          <path class="stand" d="M 9 18.5 Q 9 25.5 16 25.5 Q 23 25.5 23 18.5" />
          <line class="stand" x1="16" y1="25.5" x2="16" y2="29.5" />
          <line class="stand" x1="10" y1="33" x2="22" y2="33" />
        </svg>
        <div class="legend-text">
          <strong>Recording</strong>
          <span class="field-hint">Red capsule — microphone is on</span>
        </div>
      </div>
      <div class="legend-item">
        <svg class="mini-mic" viewBox="0 0 32 36" aria-hidden="true">
          <rect class="capsule transcribing" x="10" y="1" width="12" height="17" rx="6" />
          <path class="stand" d="M 9 18.5 Q 9 25.5 16 25.5 Q 23 25.5 23 18.5" />
          <line class="stand" x1="16" y1="25.5" x2="16" y2="29.5" />
          <line class="stand" x1="10" y1="33" x2="22" y2="33" />
        </svg>
        <div class="legend-text">
          <strong>Recognizing</strong>
          <span class="field-hint">Blue capsule — transcribing and pasting</span>
        </div>
      </div>
    </div>
  </section>

  <section class="auth-block" aria-label="Account">
    <span class="legend-heading">Account</span>
    <p class="field-hint">
      Ordinary path: sign in with SuperGrok or X Premium+
      (<code>cosmic-scribe --login</code>). An API key is a fallback.
    </p>
    <p class="field-hint auth-status">
      Connection:
      {#if authMode === 'oauth'}
        <span class="ok">signed in</span>
      {:else if authMode === 'api_key' || authMode === 'api_key_env'}
        <span class="ok">using an API key</span>
      {:else if hasKey}
        <span class="ok">API key available</span>
      {:else}
        <span class="err">not set up yet</span>
      {/if}
    </p>
  </section>
  <label>
    <span>API key</span>
    <input type="password" bind:value={apiKey} placeholder={hasKey && authMode !== 'oauth' ? '(saved on this computer)' : 'paste speech API key…'} autocomplete="off">
    <span class="field-hint">For cloud speech recognition. Environment keys take priority over a key saved here.</span>
  </label>
  <label>
    <span>Language</span>
    <input type="text" bind:value={lang} placeholder="en" autocomplete="off" spellcheck="false" />
    <span class="field-hint">Language code for recognition (en, ru, de, ja…). Default: en.</span>
  </label>
  <label>
    <span>STT endpoint</span>
    <input type="url" bind:value={sttEndpoint} placeholder="https://api.x.ai/v1/stt" autocomplete="off" spellcheck="false" />
    <span class="field-hint">Full URL for the current speech dialect (default xAI). Changing host alone is not enough for OpenAI Whisper — see docs/STT_PROVIDERS.md.</span>
  </label>
  <label>
    <span>When text is ready</span>
    <Select options={outputOptions} bind:value={outputMode} />
  </label>
  <label>
    <span>History time labels</span>
    <Select options={timeModeOptions} bind:value={historyTimeMode} />
  </label>
  <label class="analytics-row">
    <span>Usage numbers</span>
    <span class="field-hint">Off by default. Anonymous counts only — no words, audio, or who you are.</span>
    <label class="check">
      <input type="checkbox" bind:checked={analyticsOptIn} />
      Share anonymous counts with developers
    </label>
    <span class="field-hint">{analyticsSummary || 'Off — nothing is recorded.'}</span>
  </label>
  <div class="actions">
    <button class="btn btn-primary" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    <span class="status" aria-live="polite">
      {#if saved}<span class="ok">Saved</span>{/if}
      {#if error}<span class="err">{error}</span>{/if}
    </span>
  </div>
</form>

<style>
  .tray-legend {
    margin-bottom: 20px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .legend-heading {
    display: block;
    font-size: 13px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0;
    font-size: 14px;
  }
  .legend-intro { margin-bottom: 12px; }
  .legend-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    border: 1px solid var(--border-subtle);
  }
  .legend-text strong {
    display: block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 2px;
  }
  .legend-text .field-hint { margin-top: 0; }
  .mini-mic {
    width: 28px;
    height: 32px;
    flex-shrink: 0;
    display: block;
  }
  .mini-mic .capsule.idle { fill: var(--text); }
  .mini-mic .capsule.recording { fill: #dc2828; }
  .mini-mic .capsule.transcribing { fill: #378cff; }

  .mini-mic .stand {
    fill: none;
    stroke: var(--text);
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  form label { display: block; margin-bottom: 16px; }
  form label > span:first-child {
    display: block;
    font-size: 13px;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
  .actions { display: flex; align-items: center; gap: 12px; margin-top: 4px; min-height: 40px; }
  .status { min-width: 0; flex: 1; min-height: 20px; }
  .ok { color: var(--success); font-size: 13px; }
  .err { color: var(--danger); font-size: 13px; line-height: 1.4; }
</style>