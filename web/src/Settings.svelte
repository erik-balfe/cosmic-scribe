<script>
  import Combobox from './lib/Combobox.svelte';

  let lang = $state('en');
  let outputMode = $state('auto');
  let hasKey = $state(false);
  let hasCorrectionKey = $state(false);
  let correctionModel = $state('deepseek/deepseek-chat-v4');
  let saved = $state(false);
  let error = $state('');
  let models = $state([]);

  async function load() {
    const [cr, mr] = await Promise.all([
      fetch('/api/config'),
      fetch('/api/models')
    ]);
    const c = await cr.json();
    lang = c.lang || 'en';
    outputMode = c.output_mode === 'clipboard' ? 'clipboard' : 'wtype';
    hasKey = c.has_key;
    hasCorrectionKey = c.has_correction_key;
    correctionModel = c.correction_model || 'deepseek/deepseek-chat-v4';
    const m = await mr.json();
    if (Array.isArray(m)) models = m;
  }
  load();

  async function save(e) {
    e.preventDefault();
    const key = e.target.key.value;
    const ckey = e.target.correction_key.value;
    const body = { lang, output_mode: outputMode, correction_model: correctionModel };
    if (key) body.key = key;
    if (ckey) body.correction_key = ckey;
    const r = await fetch('/api/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    if (r.ok) {
      saved = true;
      e.target.key.value = '';
      e.target.correction_key.value = '';
      if (key) hasKey = true;
      if (ckey) hasCorrectionKey = true;
      setTimeout(() => saved = false, 2000);
    } else {
      error = 'Save failed';
    }
  }
</script>

<form onsubmit={save}>
  <label><span>API Key</span>
    <input type="password" name="key" placeholder={hasKey ? '(stored)' : 'sk-...'} autocomplete="off">
  </label>
  <label><span>Language</span>
    <select bind:value={lang}>
      <option value="en">English</option><option value="ru">Русский</option>
      <option value="de">Deutsch</option><option value="fr">Français</option><option value="es">Español</option>
    </select>
  </label>
  <label><span>Text output</span>
    <select bind:value={outputMode}>
      <option value="wtype">wtype — type into focused field (default)</option>
      <option value="clipboard">Clipboard — copy only, you paste (terminals)</option>
    </select>
  </label>
  <label><span>OpenRouter Key <span class="beta">beta</span></span>
    <span class="beta-hint">AI correction is experimental — marking words and “Fix with AI” may give poor results.</span>
    <input type="password" name="correction_key" placeholder={hasCorrectionKey ? '(stored)' : 'sk-or-v1-...'} autocomplete="off">
  </label>
  <label><span>Correction Model <span class="beta">beta</span></span>
    <Combobox options={models} bind:value={correctionModel} placeholder="Select model..." />
  </label>
  <button type="submit">Save</button>
  {#if saved}<span class="ok">Saved</span>{/if}
  {#if error}<span class="err">{error}</span>{/if}
</form>

<style>
  form { background: #16213e; border-radius: 12px; padding: 24px; }
  label { display: block; margin-bottom: 16px; }
  label span { display: block; font-size: 13px; color: #aaa; margin-bottom: 6px; }
  input, select {
    width: 100%; padding: 10px 14px; border-radius: 8px; border: 1px solid #333;
    background: #0f3460; color: #fff; font-size: 14px;
  }
  input:focus, select:focus { outline: none; border-color: #e94560; }
  button { background: #e94560; color: #fff; border: none; padding: 10px 20px; border-radius: 8px; font-size: 14px; cursor: pointer; }
  button:hover { background: #d63851; }
  .ok { color: #95d5b2; margin-left: 12px; font-size: 13px; }
  .err { color: #e94560; margin-left: 12px; font-size: 13px; }
  .beta {
    font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
    color: #f4a261; background: rgba(244, 162, 97, 0.15); padding: 2px 6px; border-radius: 4px;
    margin-left: 6px; vertical-align: middle;
  }
  .beta-hint { display: block; font-size: 12px; color: #888; margin: 4px 0 8px; line-height: 1.4; }
</style>
