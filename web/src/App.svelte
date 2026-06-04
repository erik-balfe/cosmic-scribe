<script>
  import Settings from './Settings.svelte';
  import History from './History.svelte';
  import Detail from './Detail.svelte';
  import Toast from './Toast.svelte';

  let route = $state({ view: 'history', id: null, version: null });

  function parsePath() {
    const p = window.location.pathname;
    const search = new URLSearchParams(window.location.search);
    if (p.startsWith('/recording/')) {
      route = { view: 'detail', id: p.split('/')[2], version: parseInt(search.get('v')) || 0 };
    } else if (p === '/settings') {
      route = { view: 'settings', id: null, version: null };
    } else {
      route = { view: 'history', id: null, version: null };
    }
  }

  function navigate(view, id = null, version = 0) {
    let url;
    if (view === 'detail' && id) {
      url = `/recording/${id}${version ? `?v=${version}` : ''}`;
    } else if (view === 'settings') {
      url = '/settings';
    } else {
      url = '/';
    }
    history.pushState(null, '', url);
    parsePath();
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('popstate', parsePath);
    parsePath();
  }
</script>

<div class="app">
  <header>
    <h1>Cosmic Scribe <span class="tagline">recordings &amp; transcripts</span></h1>
    <nav>
      <button class:active={route.view === 'history' || route.view === 'detail'} onclick={() => navigate('history')}>History</button>
      <button class:active={route.view === 'settings'} onclick={() => navigate('settings')}>Settings</button>
    </nav>
  </header>
  <main>
    {#if route.view === 'detail' && route.id}
      <Detail detailId={route.id} initialVersion={route.version} onback={() => navigate('history')} onnavigate={navigate} />
    {:else if route.view === 'settings'}
      <Settings />
    {:else}
      <History onselect={(id) => navigate('detail', id)} />
    {/if}
  </main>
  <Toast />
</div>

<style>
  :global(*) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #1a1a2e; color: #e0e0e0; padding: 20px;
  }
  .app { max-width: 720px; margin: 0 auto; }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
  h1 { font-size: 20px; color: #fff; }
  .tagline { font-size: 12px; font-weight: 400; color: #888; margin-left: 8px; }
  nav { display: flex; gap: 4px; }
  nav button {
    padding: 8px 16px; border-radius: 6px; cursor: pointer;
    border: 1px solid #333; background: none; color: #aaa; font-size: 14px;
  }
  nav button:hover { border-color: #555; color: #ddd; }
  nav button.active { background: #0f3460; color: #fff; border-color: #e94560; }
</style>
