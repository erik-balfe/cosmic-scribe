<script>
  import { copyText } from './lib/toast.js';

  let { detailId, initialVersion, onback, onnavigate } = $props();
  let data = $state(null);
  let versions = $state([]);
  let activeVersion = $state(initialVersion || 0);
  let editing = $state(false);
  let editText = $state('');
  let mode = $state('red');
  let marks = $state({});
  let correcting = $state(false);
  let errMsg = $state('');
  let audio = $state(null);
  let playing = $state(false);
  let audioTime = $state(0);
  let audioDur = $state(0);
  let showDelete = $state(false);
  let transcribing = $state(false);
  let needsTranscript = $derived(!editText?.trim());

  async function load() {
    const r = await fetch(`/api/recording/${detailId}`);
    data = await r.json();
    versions = data.versions || [];
    editText = data.text || '';
    marks = {};
    activeVersion = initialVersion || 0;
    if (activeVersion > 0 && versions[activeVersion - 1]) {
      editText = versions[activeVersion - 1].text || '';
    }
  }
  load();

  function humanTime(ts) {
    if (!ts) return '';
    const d = new Date(ts);
    if (isNaN(d.getTime())) return ts;
    return d.toLocaleString();
  }

  function switchVersion(idx) {
    activeVersion = idx;
    editText = idx === 0 ? (data.text || '') : (versions[idx - 1]?.text || '');
    marks = {};
    onnavigate?.('detail', detailId, idx);
  }

  function markSelection() {
    const sel = window.getSelection();
    if (!sel.rangeCount || sel.isCollapsed) return;
    const text = sel.toString();
    if (!text.trim()) return;

    const words = editText.split(/([\s.,!?;:'"()\-\u2014\u2013]+)/g).filter(w => w.length > 0);
    const selectedStart = editText.indexOf(text);
    if (selectedStart < 0) return;
    const selectedEnd = selectedStart + text.length;

    let pos = 0;
    for (let i = 0; i < words.length; i++) {
      const w = words[i];
      const wordStart = pos;
      const wordEnd = pos + w.length;
      if (wordEnd > selectedStart && wordStart < selectedEnd) {
        if (mode === 'red') {
          marks[i] = marks[i] === 'wrong' ? '' : 'wrong';
        } else {
          marks[i] = marks[i] === 'correct' ? '' : 'correct';
        }
      }
      pos = wordEnd;
    }
    marks = { ...marks };
    sel.removeAllRanges();
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

  async function correctWithAI() {
    const allWrong = [], allCorrect = [];
    const words = editText.split(/([\s.,!?;:'"()\-\u2014\u2013]+)/g).filter(w => w.length > 0);
    for (const [idx, type] of Object.entries(marks)) {
      if (type === 'wrong') allWrong.push(words[parseInt(idx)]);
      if (type === 'correct') allCorrect.push(words[parseInt(idx)]);
    }
    correcting = true;
    errMsg = '';
    try {
      const r = await fetch(`/api/recording/${detailId}/correct`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text: editText, marked: allWrong, kept: allCorrect })
      });
      if (r.ok) { await load(); switchVersion(versions.length); }
      else { const err = await r.json(); errMsg = err.text || err.error || 'Correction failed'; }
    } catch { errMsg = 'Network error'; }
    correcting = false;
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

  let words = $derived(editText.split(/([\s.,!?;:'"()\-\u2014\u2013]+)/g).filter(w => w.length > 0));
  let hasMarks = $derived(Object.values(marks).some(m => m));

  function formatTime(s) {
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, '0')}`;
  }
</script>

{#if data}
<div class="detail">
  <div class="topbar">
    <button class="back" onclick={() => onback?.()}>Back</button>
    <span class="ts" title={humanTime(data.ts)}>{humanTime(data.ts)}</span>
    <span class="dur">{data.duration || '?'}</span>
    <span class="grow"></span>
    <button class="play-btn" onclick={toggleAudio}>{playing ? '\u23F8' : '\u25B6'}</button>
    {#if editText?.trim()}
      <button class="copy-btn" onclick={() => copyText(editText)} title="Copy transcript">Copy</button>
    {/if}
    {#if !showDelete}
      <button class="del" onclick={() => showDelete = true}>Delete</button>
    {:else}
      <button class="del-yes" onclick={deleteEntry}>Delete</button>
      <button class="del-no" onclick={() => showDelete = false}>Keep</button>
    {/if}
  </div>

  {#if audioDur > 0}
    <div class="time-display">{formatTime(audioTime)} / {formatTime(audioDur)}</div>
  {/if}

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
    <button class="mode-btn" class:active={mode === 'red'} onclick={() => mode = 'red'}>
      <span class="dot red"></span>Wrong
    </button>
    <button class="mode-btn" class:active={mode === 'green'} onclick={() => mode = 'green'}>
      <span class="dot green"></span>Correct
    </button>
    {#if hasMarks}
      <button onclick={() => marks = {}}>Clear marks</button>
    {/if}
    <span class="grow"></span>
    <button onclick={() => editing = !editing}>{editing ? 'Preview' : 'Edit'}</button>
    {#if hasMarks}
      <button class="correct-btn" onclick={correctWithAI} disabled={correcting} title="Experimental — results vary">
        {correcting ? '...' : 'Fix with AI'} <span class="beta">beta</span>
      </button>
    {/if}
  </div>

  {#if !editing}
    <div class="hint">Select text to mark as {mode === 'red' ? 'wrong (click again to remove)' : 'correct'}</div>
  {/if}
  {#if errMsg}
    <div class="error-msg">{errMsg}</div>
  {/if}

  <div class="text-area">
    {#if editing}
      <textarea bind:value={editText} rows={8}></textarea>
      <button class="save-btn" onclick={saveEdit}>Save edit</button>
    {:else}
      <div class="markable-text" onmouseup={markSelection}>
        {#each words as word, i (`${i}-${word}`)}
          {@const m = marks[i] || ''}
          <span class="word" class:wrong={m === 'wrong'} class:correct={m === 'correct'}>{word}</span>
        {/each}
      </div>
    {/if}
  </div>
</div>
{/if}

<style>
  .detail { background: #16213e; border-radius: 12px; padding: 20px; }
  .topbar { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  .back { background: none; border: none; color: #aed6f1; cursor: pointer; font-size: 14px; padding: 0; }
  .back:hover { color: #fff; }
  .ts { font-size: 12px; color: #888; white-space: nowrap; }
  .dur { font-size: 11px; color: #666; }
  .grow { flex: 1; }
  .play-btn { background: #0f3460; border: 1px solid #2471a3; color: #aed6f1; border-radius: 6px;
    padding: 4px 12px; cursor: pointer; font-size: 14px; width: 40px; text-align: center; }
  .play-btn:hover { background: #1a5276; }
  .copy-btn {
    background: #0f3460; border: 1px solid #2471a3; color: #aed6f1; border-radius: 6px;
    padding: 4px 12px; cursor: pointer; font-size: 13px;
  }
  .copy-btn:hover { background: #1a5276; color: #fff; }
  .del, .del-yes, .del-no {
    font-size: 12px; padding: 4px 10px; border-radius: 4px; border: 1px solid #333;
    background: none; cursor: pointer; min-width: 52px; text-align: center;
  }
  .del { color: #888; }
  .del:hover { color: #e94560; border-color: #e94560; }
  .del-yes { color: #e94560; border-color: #e94560; }
  .del-no { color: #888; }

  .time-display { text-align: center; font-size: 12px; color: #888; margin-bottom: 8px; }

  .waveform-container { margin-bottom: 16px; cursor: pointer; }
  .waveform { display: flex; align-items: flex-end; gap: 2px; height: 50px; overflow: hidden; }
  .bar { flex: 1; background: #e94560; border-radius: 2px 2px 0 0; min-width: 2px; transition: opacity 0.15s; }

  .versions-scroll { overflow-x: auto; margin-bottom: 12px; }
  .versions { display: flex; gap: 4px; }
  .versions button {
    padding: 5px 10px; border-radius: 6px; border: 1px solid #333;
    background: none; color: #aaa; cursor: pointer; font-size: 12px; white-space: nowrap;
  }
  .versions button.active { background: #0f3460; color: #fff; border-color: #e94560; }

  .toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; flex-wrap: wrap; }
  .toolbar button {
    padding: 6px 12px; border-radius: 6px; border: 1px solid #333;
    background: none; color: #aaa; cursor: pointer; font-size: 13px;
  }
  .toolbar button:hover { border-color: #555; }
  .mode-btn { display: flex; align-items: center; gap: 6px; }
  .mode-btn.active { background: #0f3460; border-color: #555; color: #fff; }
  .dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
  .dot.red { background: #e94560; }
  .dot.green { background: #4caf50; }
  .correct-btn { background: #1a5276 !important; border-color: #2471a3 !important; color: #aed6f1 !important; }
  .correct-btn:disabled { opacity: 0.4; }
  .beta {
    font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
    color: #f4a261; background: rgba(244, 162, 97, 0.2); padding: 1px 5px; border-radius: 3px;
    margin-left: 4px; vertical-align: middle;
  }

  .no-transcript {
    background: #1a2a4a; border: 1px solid #334;
    border-radius: 8px; padding: 14px 16px; margin-bottom: 14px;
  }
  .no-transcript p { margin: 0 0 10px; font-size: 13px; color: #aaa; line-height: 1.5; }
  .transcribe-btn {
    background: #1a5276; border: 1px solid #2471a3; color: #aed6f1;
    border-radius: 6px; padding: 8px 16px; cursor: pointer; font-size: 13px;
  }
  .transcribe-btn:hover:not(:disabled) { background: #2471a3; color: #fff; }
  .transcribe-btn:disabled { opacity: 0.5; cursor: wait; }

  .hint { font-size: 12px; color: #666; margin-bottom: 8px; }
  .error-msg { font-size: 13px; color: #e94560; background: #3d1a1a; padding: 8px 12px; border-radius: 6px; margin-bottom: 8px; }

  .markable-text { line-height: 1.9; font-size: 15px; color: #ccc; user-select: text; }
  .markable-text::selection { background: rgba(233, 69, 96, 0.25); }
  .word { padding: 1px 2px; border-radius: 3px; }
  .word.wrong { background: rgba(233, 69, 96, 0.35); }
  .word.correct { background: rgba(76, 175, 80, 0.3); }

  textarea {
    width: 100%; padding: 12px; border-radius: 8px; border: 1px solid #333;
    background: #0f3460; color: #fff; font-size: 15px; line-height: 1.6;
    resize: vertical; font-family: inherit;
  }
  textarea:focus { outline: none; border-color: #e94560; }
  .save-btn { margin-top: 8px; background: #1b4f3a; color: #95d5b2; border: none;
    padding: 8px 16px; border-radius: 6px; cursor: pointer; }
</style>
