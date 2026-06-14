<script>
  import Combobox from './lib/Combobox.svelte';
  import Select from './lib/Select.svelte';

  let { historyTimeMode = $bindable('relative'), onsaved } = $props();

  let lang = $state('en');
  let outputMode = $state('wtype');
  let hasKey = $state(false);
  let hasCorrectionKey = $state(false);
  let correctionModel = $state('deepseek/deepseek-chat-v4');
  let saved = $state(false);
  let error = $state('');
  let saving = $state(false);
  let models = $state([]);
  let apiKey = $state('');
  let correctionKey = $state('');

  const outputOptions = [
    { value: 'wtype', label: 'wtype — type into focused field (default)' },
    { value: 'clipboard', label: 'Clipboard — copy only, you paste (terminals)' },
  ];

  const timeModeOptions = [
    { value: 'relative', label: 'Relative — e.g. 2h ago (hover for exact time)' },
    { value: 'absolute', label: 'Absolute — e.g. Jun 6, 9:15 PM (hover for relative)' },
  ];

  async function load() {
    error = '';
    try {
      const [cr, mr] = await Promise.all([
        fetch('/api/config'),
        fetch('/api/models'),
      ]);
      if (!cr.ok) {
        error = `Failed to load settings (${cr.status})`;
        return;
      }
      const c = await cr.json();
      lang = c.lang || 'en';
      outputMode = c.output_mode === 'clipboard' ? 'clipboard' : 'wtype';
      historyTimeMode = c.history_time_mode === 'absolute' ? 'absolute' : 'relative';
      hasKey = c.has_key;
      hasCorrectionKey = c.has_correction_key;
      correctionModel = c.correction_model || 'deepseek/deepseek-chat-v4';
      if (mr.ok) {
        const m = await mr.json();
        if (Array.isArray(m)) models = m;
      }
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
      correction_model: correctionModel,
    };
    if (apiKey) body.key = apiKey;
    if (correctionKey) body.correction_key = correctionKey;
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
        correctionKey = '';
        if (body.key) hasKey = true;
        if (body.correction_key) hasCorrectionKey = true;
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

  <label>
    <span>API Key</span>
    <input type="password" bind:value={apiKey} placeholder={hasKey ? '(stored)' : 'sk-...'} autocomplete="off">
  </label>
  <label>
    <span>Language</span>
    <input type="text" bind:value={lang} placeholder="en" autocomplete="off" spellcheck="false" />
    <span class="field-hint">Code sent to xAI STT (e.g. en, de, ja). Default is en.</span>
  </label>
  <label>
    <span>Text output</span>
    <Select options={outputOptions} bind:value={outputMode} />
  </label>
  <label>
    <span>History timestamps</span>
    <Select options={timeModeOptions} bind:value={historyTimeMode} />
  </label>
  <label>
    <span>OpenRouter Key <span class="beta">beta</span></span>
    <span class="field-hint beta-hint">AI correction is experimental — marking words and “Fix with AI” may give poor results.</span>
    <input type="password" bind:value={correctionKey} placeholder={hasCorrectionKey ? '(stored)' : 'sk-or-v1-...'} autocomplete="off">
  </label>
  <label>
    <span>Correction Model <span class="beta">beta</span></span>
    <Combobox options={models} bind:value={correctionModel} placeholder="Select model..." />
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
  .beta-hint { margin: 4px 0 8px; }
  .actions { display: flex; align-items: center; gap: 12px; margin-top: 4px; min-height: 40px; }
  .status { min-width: 0; flex: 1; min-height: 20px; }
  .ok { color: var(--success); font-size: 13px; }
  .err { color: var(--danger); font-size: 13px; line-height: 1.4; }
</style>