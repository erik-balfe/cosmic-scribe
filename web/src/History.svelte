<script>
  import { copyText } from './lib/toast.js';
  import { formatDuration, recordingTimeLabels } from './lib/time.js';
  import { mergeHistoryEntries } from './lib/history.js';

  let { onselect, live = false, timeMode = 'relative' } = $props();
  let entries = $state([]);
  let offset = $state(0);
  let hasMore = $state(true);
  let loading = $state(false);
  let loadingMore = $state(false);
  let confirmingDelete = $state(null);

  const POLL_MS = 2000;

  async function fetchPage(pageOffset, limit = 20) {
    const r = await fetch(`/api/history?offset=${pageOffset}&limit=${limit}`);
    if (!r.ok) return [];
    return r.json();
  }

  async function loadInitial() {
    loading = true;
    const data = await fetchPage(0);
    entries = data;
    offset = data.length;
    hasMore = data.length >= 20;
    loading = false;
  }

  async function loadMore() {
    if (loadingMore || !hasMore) return;
    loadingMore = true;
    const data = await fetchPage(offset);
    if (data.length < 20) hasMore = false;
    entries = [...entries, ...data];
    offset += data.length;
    loadingMore = false;
  }

  async function refreshLatest() {
    if (document.hidden) return;
    const fresh = await fetchPage(0, Math.max(20, entries.length || 20));
    if (!fresh.length && !entries.length) return;
    entries = mergeHistoryEntries(entries, fresh);
    if (offset < fresh.length) offset = fresh.length;
  }

  loadInitial();

  $effect(() => {
    if (!live || typeof window === 'undefined') return;
    const id = setInterval(refreshLatest, POLL_MS);
    const onVisible = () => {
      if (!document.hidden) refreshLatest();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
    };
  });

  function truncate(text, max = 120) {
    if (!text) return '';
    if (text.length <= max) return text;
    const half = Math.floor(max / 2);
    return text.slice(0, half) + '\u2026' + text.slice(text.length - half);
  }

  async function deleteEntry(file, idx) {
    await fetch(`/api/recording/${file}/delete`, { method: 'POST' });
    entries = entries.filter((_, i) => i !== idx);
    confirmingDelete = null;
  }
</script>

<div class="list">
  {#each entries as e, i (e.file)}
    {@const time = recordingTimeLabels(e.ts, timeMode)}
    <div
      class="entry"
      onclick={() => onselect?.(e.file)}
      role="button"
      tabindex="0"
      onkeydown={(ev) => ev.key === 'Enter' && onselect?.(e.file)}
    >
      <div class="recording-header entry-header">
        <div class="recording-meta">
          <span class="ts" title={time.tooltip}>{time.primary}</span>
          <span class="dur">{formatDuration(e.duration)}</span>
        </div>
        <div
          class="action-toolbar"
          onclick={(ev) => ev.stopPropagation()}
          onkeydown={(ev) => ev.stopPropagation()}
          role="presentation"
        >
          <button
            class="btn toolbar-slot"
            disabled={!e.has_text}
            onclick={() => copyText(e.text)}
            title={e.has_text ? 'Copy transcript' : 'No transcript to copy'}
          >Copy</button>
          <div class="toolbar-confirm" class:is-confirming={confirmingDelete === i}>
            <button
              class="btn btn-danger toolbar-slot delete-primary"
              onclick={() => confirmingDelete === i ? deleteEntry(e.file, i) : (confirmingDelete = i)}
            >{confirmingDelete === i ? 'Confirm' : 'Delete'}</button>
            <button
              class="btn toolbar-slot cancel-slot"
              tabindex={confirmingDelete === i ? 0 : -1}
              aria-hidden={confirmingDelete !== i}
              onclick={() => confirmingDelete = null}
            >Cancel</button>
          </div>
        </div>
      </div>
      <div class="preview">
        {#if e.has_text}
          {truncate(e.text)}
        {:else}
          <span class="no-text">No transcript</span>
        {/if}
      </div>
    </div>
  {/each}

  {#if hasMore}
    <button class="btn more" onclick={loadMore} disabled={loadingMore}>
      {loadingMore ? 'Loading…' : 'Show more'}
    </button>
  {/if}

  {#if loading && entries.length === 0}
    <div class="panel-loading">Loading recordings…</div>
  {:else if entries.length === 0}
    <div class="empty">No recordings yet.</div>
  {/if}
</div>

<style>
  .list { display: flex; flex-direction: column; gap: 10px; }
  .entry {
    background: var(--surface);
    border-radius: var(--radius);
    border: 1px solid var(--border-subtle);
    padding: 14px 16px;
    cursor: pointer;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }
  .entry:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-soft), 0 4px 16px rgba(0, 0, 0, 0.18);
  }
  .entry:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-color: var(--accent);
  }
  .entry-header {
    margin-bottom: 8px;
  }
  .preview {
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1.45;
    min-height: 1.45em;
  }
  .no-text { color: var(--accent); font-style: italic; }
  .more {
    width: 100%;
    padding: 12px;
    margin-top: 4px;
    color: var(--blue);
    background: var(--surface);
    min-height: 44px;
  }
  .more:disabled { opacity: 0.5; cursor: default; }
  .empty { text-align: center; color: var(--text-dim); padding: 48px 20px; }
</style>