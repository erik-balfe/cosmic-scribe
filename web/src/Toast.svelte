<script>
  import { registerToast } from './lib/toast.js';

  let message = $state(null);
  let timer;

  registerToast((msg) => {
    message = msg;
    clearTimeout(timer);
    timer = setTimeout(() => { message = null; }, 2200);
  });
</script>

{#if message}
  <div class="toast" role="status">{message}</div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--success-bg);
    color: var(--success);
    padding: 10px 18px;
    border-radius: 8px;
    font-size: 14px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    z-index: 1000;
    pointer-events: none;
  }
</style>