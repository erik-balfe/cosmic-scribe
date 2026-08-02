<script>
  import { copyText } from './lib/toast.js';
  import { formatDuration, recordingTimeLabels } from './lib/time.js';

  let { detailId, initialVersion, onback, onnavigate, timeMode = 'relative' } = $props();
  let data = $state(null);
  let versions = $state([]);
  let activeVersion = $state(initialVersion || 0);
  let editing = $state(false);
  let editText = $state('');
  let errMsg = $state('');
  let audio = $state(null);
  let playing = $state(false);
  let audioTime = $state(0);
  let audioDur = $state(0);
  let showDelete = $state(false);
  let transcribing = $state(false);
  let loadError = $state('');
  let needsTranscript = $derived(!editText?.trim());

  async function load() {
    loadError = '';
    const r = await fetch(`/api/recording/${detailId}`);
    if (!r.ok) {
      loadError = r.status === 404 ? 'Recording not found.' : `Failed to load (${r.status}).`;
      data = null;
      return;
    }
    data = await r.json();
    versions = data.versions || [];
    editText = data.text || '';
    activeVersion = initialVersion || 0;
    if (activeVersion > 0 && versions[activeVersion - 1]) {
      editText = versions[activeVersion - 1].text || '';
    }
  }
  load();

  function switchVersion(idx) {
    activeVersion = idx;
    editText = idx === 0 ? (data.text || '') : (versions[idx - 1]?.text || '');
    onnavigate?.('detail', detailId, idx);
  }

  async function saveEdit() {
    const r = await fetch(`/api/recording/${detailId}/edit`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: editText, type: 'user_edit' })
    });
    if (r.ok) {
      editing = false;
      await load();
      const latestVersion = versions.length;
      switchVersion(latestVersion);
      requestAnimationFrame(() => {
        const el = document.querySelector('.versions-scroll');
        if (el) el.scrollLeft = el.scrollWidth;
      });
    }
  }

  function deleteEntry() {
    fetch(`/api/recording/${detailId}/delete`, { method: 'POST' }).then(() => onback?.());
  }

  async function transcribeRecording() {
    transcribing = true;
    errMsg = '';
    try {
      const r = await fetch(`/api/recording/${detailId}/transcribe`, { method: 'POST' });
      const body = await r.json();
      if (r.ok && body.ok) {
        await load();
        switchVersion(0);
      } else {
        errMsg = body.error || 'Transcription failed';
      }
    } catch {
      errMsg = 'Network error';
    }
    transcribing = false;
  }

  function toggleAudio() {
    if (!audio) {
      const a = new Audio(`/api/recording/${detailId}/audio`);
      a.addEventListener('timeupdate', () => audioTime = a.currentTime);
      a.addEventListener('loadedmetadata', () => { audioDur = a.duration; a.play(); playing = true; });
      a.addEventListener('ended', () => { playing = false; audioTime = 0; });
      a.addEventListener('error', () => { playing = false; });
      audio = a;
    } else if (playing) { audio.pause(); playing = false; }
    else { audio.play(); playing = true; }
  }

  function seekWaveform(e) {
    if (!audio || !audioDur) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const pct = (e.clientX - rect.left) / rect.width;
    audio.currentTime = pct * audioDur;
  }

  let timeLabels = $derived(data ? recordingTimeLabels(data.ts, timeMode) : { primary: '', tooltip: '' });

  function formatTime(s) {
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, '0')}`;
  }
</script>

{#if loadError}
  <div class="panel detail">
    <div class="detail-nav">
      <button class="btn btn-ghost back" onclick={() => onback?.()} aria-label="Back to history">
        <span aria-hidden="true">←</span> Back to history
      </button>
    </div>
    <div class="error-msg">{loadError}</div>
  </div>
{:else if !data}
  <div class="panel-loading">Loading recording…</div>
{:else}
<div class="detail">
  <div class="detail-nav">
    <button class="btn btn-ghost back" onclick={() => onback?.()} aria-label="Back to history">
      <span aria-hidden="true">←</span> Back to history
    </button>
  </div>
  <div class="recording-header">
    <div class="recording-meta">
      <span class="ts" title={timeLabels.tooltip}>{timeLabels.primary}</span>
      <span class="dur">{formatDuration(data.duration)}</span>
    </div>
    <div class="action-toolbar">
      <button
        class="btn toolbar-slot play"
        onclick={toggleAudio}
        title={playing ? 'Pause' : 'Play audio'}
      >
        {playing ? 'Pause' : 'Play'}
      </button>
      <button
        class="btn toolbar-slot"
        disabled={!editText?.trim()}
        onclick={() => copyText(editText)}
        title={editText?.trim() ? 'Copy transcript' : 'No transcript to copy'}
      >Copy</button>
      <div class="toolbar-confirm" class:is-confirming={showDelete}>
        <button
          class="btn btn-danger toolbar-slot delete-primary"
          onclick={() => { if (showDelete) deleteEntry(); else showDelete = true; }}
        >{showDelete ? 'Confirm' : 'Delete'}</button>
        <button
          class="btn toolbar-slot cancel-slot"
          tabindex={showDelete ? 0 : -1}
          aria-hidden={!showDelete}
          onclick={() => showDelete = false}
        >Cancel</button>
      </div>
    </div>
  </div>

  <div class="playback-time" aria-live="polite">
    {audioDur > 0 ? `${formatTime(audioTime)} / ${formatTime(audioDur)}` : '\u00a0'}
  </div>

  {#if needsTranscript}
    <div class="no-transcript">
      <p>Audio saved — no transcript yet. Listen above, then transcribe when you're back online.</p>
      <button class="transcribe-btn" onclick={transcribeRecording} disabled={transcribing}>
        {transcribing ? 'Transcribing…' : 'Transcribe'}
      </button>
    </div>
  {/if}

  {#if data.waveform?.length}
    <div class="waveform-container" onclick={seekWaveform} role="button" tabindex="0">
      <div class="waveform">
        {#each data.waveform as bar, i (i)}
          <div class="bar" style="height:{Math.max(2, bar * 50)}px; opacity:{audio && audioDur ? (i/data.waveform.length <= audioTime/(audioDur||1) ? 1 : 0.35) : 1}"></div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="versions-scroll">
    <div class="versions">
      <button class:active={activeVersion === 0} onclick={() => switchVersion(0)}>Original</button>
      {#each versions as v, i (i)}
        <button class:active={activeVersion === i + 1} onclick={() => switchVersion(i + 1)}>
          {v.type === 'user_edit' ? 'Edit' : 'AI'} {i + 1}
        </button>
      {/each}
    </div>
  </div>

  <div class="toolbar">
    <span class="grow"></span>
    <button onclick={() => editing = !editing}>{editing ? 'Preview' : 'Edit'}</button>
  </div>

  {#if errMsg}
    <div class="error-msg">{errMsg}</div>
  {/if}

  <div class="text-area">
    {#if editing}
      <textarea bind:value={editText} rows={8}></textarea>
      <button class="save-btn" onclick={saveEdit}>Save edit</button>
    {:else}
      <div class="plain-text">
        {editText || 'No transcript yet.'}
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  .detail {
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: 20px;
    box-shadow: var(--shadow);
  }
  .detail-nav {
    margin: -4px 0 12px;
    padding-bottom: 4px;
  }
  .back {
    padding: 8px 12px;
    gap: 8px;
    font-size: 14px;
    font-weight: 500;
  }
  .waveform-container { margin-bottom: 16px; cursor: pointer; }
  .waveform { display: flex; align-items: flex-end; gap: 2px; height: 50px; overflow: hidden; }
  .bar { flex: 1; background: var(--accent); border-radius: 2px 2px 0 0; min-width: 2px; transition: opacity 0.15s; }

  .versions-scroll { overflow-x: auto; margin-bottom: 12px; }
  .versions { display: flex; gap: 4px; }
  .versions button {
    padding: 5px 10px; border-radius: var(--radius-sm); border: 1px solid var(--border-subtle);
    background: none; color: var(--text-muted); cursor: pointer; font-size: 12px; white-space: nowrap;
  }
  .versions button.active { background: var(--blue-soft); color: var(--text); border-color: var(--accent); }

  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; flex-wrap: wrap; }
  .toolbar button {
    padding: 6px 12px; border-radius: var(--radius-sm); border: 1px solid var(--border-subtle);
    background: none; color: var(--text-muted); cursor: pointer; font-size: 13px;
  }
  .toolbar button:hover { border-color: var(--border); color: var(--text); }

  .no-transcript {
    background: var(--surface-raised);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 14px 16px;
    margin-bottom: 14px;
  }
  .no-transcript p { margin: 0 0 10px; font-size: 13px; color: var(--text-muted); line-height: 1.5; }
  .transcribe-btn {
    background: var(--blue-soft);
    border: 1px solid var(--border);
    color: var(--blue);
    border-radius: var(--radius-sm);
    padding: 8px 16px;
    cursor: pointer;
    font-size: 13px;
  }
  .transcribe-btn:hover:not(:disabled) { background: var(--border); color: var(--text); }
  .transcribe-btn:disabled { opacity: 0.5; cursor: wait; }

  .hint { font-size: 12px; color: var(--text-dim); margin-bottom: 8px; }
  .error-msg {
    font-size: 13px; color: var(--danger); background: rgba(255, 107, 122, 0.12);
    border: 1px solid rgba(255, 107, 122, 0.25);
    padding: 8px 12px; border-radius: var(--radius-sm); margin-bottom: 8px;
  }

  .plain-text { line-height: 1.7; font-size: 15px; color: var(--text-muted); white-space: pre-wrap; }

  textarea {
    width: 100%; padding: 12px; border-radius: var(--radius-sm); border: 1px solid var(--border);
    background: var(--surface-raised); color: var(--text); font-size: 15px; line-height: 1.6;
    resize: vertical; font-family: inherit;
  }
  textarea:focus { outline: none; border-color: var(--accent); }
  .save-btn {
    margin-top: 8px; background: var(--success-bg); color: var(--success); border: 1px solid #2d5a45;
    padding: 8px 16px; border-radius: var(--radius-sm); cursor: pointer;
  }
</style>
