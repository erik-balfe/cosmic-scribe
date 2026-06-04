<script>
  let { options = [], value = $bindable(), placeholder = 'Select...' } = $props();
  let open = $state(false);
  let filter = $state('');
  let active = $state(0);
  let inputEl = $state(null);
  let listEl = $state(null);

  let filtered = $derived(
    filter
      ? options.filter(o => o.name.toLowerCase().includes(filter.toLowerCase()) || o.id.toLowerCase().includes(filter.toLowerCase()))
      : options
  );
  let selected = $derived(options.find(o => o.id === value));

  function select(id) {
    value = id;
    open = false;
    filter = '';
    active = 0;
  }

  function toggle() {
    open = !open;
    if (open) setTimeout(() => inputEl?.focus(), 50);
  }

  function handleKey(e) {
    if (e.key === 'ArrowDown') { active = Math.min(active + 1, filtered.length - 1); e.preventDefault(); }
    else if (e.key === 'ArrowUp') { active = Math.max(active - 1, 0); e.preventDefault(); }
    else if (e.key === 'Enter' && filtered[active]) { select(filtered[active].id); e.preventDefault(); }
    else if (e.key === 'Escape') { open = false; }
  }

  function handleClickOutside(e) {
    if (!e.target.closest('.combobox')) open = false;
  }

  $effect(() => {
    if (open) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  $effect(() => {
    if (open && listEl && active >= 0) {
      const item = listEl.children[active];
      if (item) item.scrollIntoView({ block: 'nearest' });
    }
  });
</script>

<div class="combobox" role="combobox" aria-expanded={open}>
  <button class="trigger" onclick={toggle} type="button">
    <span>{selected?.name || placeholder}</span>
    <span class="arrow">{open ? '▴' : '▾'}</span>
  </button>
  {#if open}
    <div class="dropdown">
      <input
        class="search"
        type="text"
        placeholder="Type to search..."
        bind:value={filter}
        onkeydown={handleKey}
        bind:this={inputEl}
      >
      <div class="list" bind:this={listEl}>
        {#each filtered as opt, i (opt.id)}
          {@const rec = opt.rec ?? false}
          <button
            class="opt"
            class:active={i === active}
            class:selected={opt.id === value}
            class:rec
            onmousedown={() => select(opt.id)}
            type="button"
          >
            <span>{rec ? '★ ' : ''}{opt.name}</span>
            <span class="pricing">{opt.pricing}</span>
          </button>
        {:else}
          <div class="empty">No models match</div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .combobox { position: relative; }
  .trigger {
    width: 100%; padding: 10px 14px; border-radius: 8px; border: 1px solid #333;
    background: #0f3460; color: #fff; font-size: 14px; cursor: pointer;
    display: flex; justify-content: space-between; align-items: center; text-align: left;
  }
  .trigger:hover { border-color: #555; }
  .trigger:focus { outline: none; border-color: #e94560; }
  .arrow { color: #888; font-size: 10px; margin-left: 8px; }
  .dropdown {
    position: absolute; top: 100%; left: 0; right: 0; z-index: 100;
    background: #16213e; border: 1px solid #333; border-radius: 8px; margin-top: 4px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }
  .search {
    width: 100%; padding: 10px 14px; border: none; border-bottom: 1px solid #2a2a4a;
    background: #0f3460; color: #fff; font-size: 14px; border-radius: 8px 8px 0 0;
  }
  .search:focus { outline: none; }
  .list { max-height: 280px; overflow-y: auto; }
  .opt {
    display: flex; justify-content: space-between; align-items: center;
    width: 100%; padding: 8px 14px; border: none; background: none;
    color: #ccc; font-size: 13px; cursor: pointer; text-align: left;
  }
  .opt:hover, .opt.active { background: #0f3460; }
  .opt.selected { color: #fff; }
  .opt.rec { color: #f0c040; }
  .pricing { font-size: 11px; color: #666; margin-left: 12px; white-space: nowrap; }
  .empty { padding: 16px; text-align: center; color: #666; font-size: 13px; }
</style>
