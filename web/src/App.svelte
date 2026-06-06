<script>
  import { fade } from 'svelte/transition';
  import Settings from './Settings.svelte';
  import History from './History.svelte';
  import Detail from './Detail.svelte';
  import Toast from './Toast.svelte';

  let route = $state({ view: 'history', id: null, version: null });

  function routeFromPath(pathname, search) {
    if (pathname.startsWith('/recording/')) {
      return {
        view: 'detail',
        id: pathname.split('/')[2],
        version: parseInt(search.get('v')) || 0,
      };
    }
    if (pathname === '/settings') {
      return { view: 'settings', id: null, version: null };
    }
    return { view: 'history', id: null, version: null };
  }

  function initRoute() {
    if (typeof window === 'undefined') return;
    const search = new URLSearchParams(window.location.search);
    route = routeFromPath(window.location.pathname, search);
  }

  function syncUrl(view, id = null, version = 0) {
    if (typeof window === 'undefined' || !window.history?.pushState) return;
    let url;
    if (view === 'detail' && id) {
      url = `/recording/${id}${version ? `?v=${version}` : ''}`;
    } else if (view === 'settings') {
      url = '/settings';
    } else {
      url = '/';
    }
    if (window.location.pathname + window.location.search !== url) {
      history.pushState(null, '', url);
    }
  }

  let historyScrollY = 0;
  let restoreHistoryScroll = false;

  function navigate(view, id = null, version = 0) {
    const from = route.view;

    if (from === 'history' && view === 'detail') {
      historyScrollY = window.scrollY;
      restoreHistoryScroll = true;
    }
    if (view === 'settings') {
      restoreHistoryScroll = false;
    }

    route = { view, id, version };
    syncUrl(view, id, version);
    requestAnimationFrame(() => {
      if (view === 'detail' || view === 'settings') {
        window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
      } else if (view === 'history') {
        const y = restoreHistoryScroll && from === 'detail' ? historyScrollY : 0;
        window.scrollTo({ top: y, left: 0, behavior: 'instant' });
        if (from === 'detail') restoreHistoryScroll = false;
      }
    });
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('popstate', initRoute);
    initRoute();
  }

  let historyActive = $derived(route.view === 'history' || route.view === 'detail');
  let settingsActive = $derived(route.view === 'settings');

  let historyTimeMode = $state('relative');

  async function loadUiPrefs() {
    try {
      const r = await fetch('/api/config');
      if (!r.ok) return;
      const c = await r.json();
      historyTimeMode = c.history_time_mode === 'absolute' ? 'absolute' : 'relative';
    } catch { /* keep default */ }
  }

  if (typeof window !== 'undefined') {
    loadUiPrefs();
  }
</script>

<div class="app">
  <header class="app-header">
    <nav class="tabs" role="tablist" aria-label="Main navigation">
      <button
        id="tab-history"
        class="tab"
        role="tab"
        aria-selected={historyActive}
        aria-controls="panel-history"
        class:active={historyActive}
        onclick={() => navigate('history')}
      >History</button>
      <button
        id="tab-settings"
        class="tab"
        role="tab"
        aria-selected={settingsActive}
        aria-controls="panel-settings"
        class:active={settingsActive}
        onclick={() => navigate('settings')}
      >Settings</button>
    </nav>
  </header>

  <main class="main">
    <div class="view-stack">
      <div
        id="panel-history"
        class="view-pane"
        role="tabpanel"
        aria-labelledby="tab-history"
        class:active={route.view === 'history'}
        aria-hidden={route.view !== 'history'}
      >
        <History
          onselect={(id) => navigate('detail', id)}
          live={route.view === 'history'}
          timeMode={historyTimeMode}
        />
      </div>

      <div
        id="panel-settings"
        class="view-pane"
        role="tabpanel"
        aria-labelledby="tab-settings"
        class:active={route.view === 'settings'}
        aria-hidden={route.view !== 'settings'}
      >
        <Settings
          bind:historyTimeMode
          onsaved={() => loadUiPrefs()}
        />
      </div>

      <div
        class="view-pane view-pane-detail"
        role="region"
        aria-label="Recording detail"
        class:active={route.view === 'detail'}
        aria-hidden={route.view !== 'detail'}
      >
        {#if route.id}
          {#key route.id}
            <div class="view-content" in:fade={{ duration: 160 }}>
              <Detail
                detailId={route.id}
                initialVersion={route.version}
                timeMode={historyTimeMode}
                onback={() => navigate('history')}
                onnavigate={navigate}
              />
            </div>
          {/key}
        {/if}
      </div>
    </div>
  </main>
  <Toast />
</div>

<style>
  .app { max-width: 760px; margin: 0 auto; }
  .app-header {
    position: sticky;
    top: 0;
    z-index: 100;
    margin: -20px -20px 0;
    padding: 12px 20px 0;
    background: var(--bg);
  }

  .tabs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 16px;
    width: 100%;
  }
  .tab {
    padding: 8px 16px 10px;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: transparent;
    color: var(--text-dim);
    font-size: 13px;
    font-weight: 500;
    line-height: 1.2;
    min-height: 36px;
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast),
      border-color var(--transition-fast);
  }
  .tab:hover {
    color: var(--text-muted);
    background: rgba(255, 255, 255, 0.04);
  }
  .tab.active {
    color: var(--text);
    background: var(--surface);
    border-bottom-color: var(--accent);
  }
  .tab:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .main { width: 100%; }
  .view-stack { position: relative; width: 100%; }
  .view-pane {
    width: 100%;
    opacity: 0;
    visibility: hidden;
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    max-height: 0;
    overflow: hidden;
    pointer-events: none;
    transition: opacity var(--transition-normal), visibility var(--transition-normal);
  }
  .view-pane.active {
    opacity: 1;
    visibility: visible;
    position: relative;
    max-height: none;
    overflow: visible;
    pointer-events: auto;
  }
  .view-content { width: 100%; }
</style>