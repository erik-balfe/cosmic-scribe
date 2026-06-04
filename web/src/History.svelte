<script>
  import { copyText } from './lib/toast.js';

  let { onselect } = $props();
  let entries = $state([]);
  let offset = $state(0);
  let hasMore = $state(true);
  let loading = $state(false);
  let confirmingDelete = $state(null);

  async function load() {
    loading = true;
    const r = await fetch(`/api/history?offset=${offset}&limit=20`);
    const data = await r.json();
    if (data.length < 20) hasMore = false;
    entries = [...entries, ...data];
    offset += data.length;
    loading = false;
  }
  load();

  function relativeTime(ts) {
    if (!ts) return '';
    const d = new Date(ts);
    if (isNaN(d.getTime())) return ts;
    const now = new Date();
    const diff = now - d;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(diff / 3600000);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(diff / 86400000);
    if (days < 7) return `${days}d ago`;
    return d.toLocaleDateString();
  }

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
    <div class="entry" onclick={() => onselect?.(e.file)} role="button" tabindex="0" onkeydown={(ev) => ev.key === 'Enter' && onselect?.(e.file)}>
      <div class="meta-row">
        <span class="time" title={e.ts}>{relativeTime(e.ts)}</span>
        <span class="dur">{e.duration}</span>
        <span class="grow"></span>
        {#if e.has_text}
          <button class="copy" onclick={(ev) => { ev.stopPropagation(); copyText(e.text); }} title="Copy transcript">Copy</button>
        {/if}
        {#if confirmingDelete === i}
          <button class="del-yes" onclick={(ev) => { ev.stopPropagation(); deleteEntry(e.file, i); }}>Delete</button>
          <button class="del-no" onclick={(ev) => { ev.stopPropagation(); confirmingDelete = null; }}>Keep</button>
        {:else}
          <button class="del" onclick={(ev) => { ev.stopPropagation(); confirmingDelete = i; }} title="Delete">&#x1F5D1;</button>
        {/if}
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
    <button class="more" onclick={load} disabled={loading}>
      {loading ? 'Loading...' : 'Show more'}
    </button>
  {/if}
  {#if entries.length === 0}
    <div class="empty">No recordings yet.</div>
  {/if}
</div>

<style>
  .list { display: flex; flex-direction: column; gap: 8px; }
  .entry {
    background: #16213e; border-radius: 10px; border: 1px solid #2a2a4a;
    padding: 14px 16px; cursor: pointer; transition: border-color 0.15s;
  }
  .entry:hover { border-color: #e94560; }
  .meta-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
  .time { font-size: 12px; color: #888; }
  .dur { font-size: 11px; color: #666; }
  .grow { flex: 1; }
  .copy {
    font-size: 12px; padding: 3px 10px; border-radius: 4px; border: 1px solid #333;
    background: none; color: #aed6f1; cursor: pointer;
  }
  .copy:hover { border-color: #2471a3; color: #fff; }
  .del, .del-yes, .del-no {
    font-size: 12px; padding: 3px 10px; border-radius: 4px; border: 1px solid #333;
    background: none; cursor: pointer; min-width: 52px; text-align: center;
  }
  .del { color: #888; font-size: 13px; padding: 2px 6px; min-width: auto; }
  .del:hover { color: #e94560; border-color: #e94560; }
  .del-yes { color: #e94560; border-color: #e94560; }
  .del-no { color: #888; }
  .preview { font-size: 13px; color: #aaa; line-height: 1.4; }
  .no-text { color: #e94560; font-style: italic; }
  .more {
    background: #0f3460; color: #aed6f1; border: 1px solid #1a5276;
    padding: 10px; border-radius: 8px; cursor: pointer; font-size: 14px;
    text-align: center; margin-top: 8px;
  }
  .more:hover { background: #1a5276; }
  .more:disabled { opacity: 0.5; cursor: default; }
  .empty { text-align: center; color: #666; padding: 40px; }
</style>
