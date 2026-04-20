<script lang="ts">
  import { appState } from '$lib/state.svelte';

  $effect(() => {
    if (!appState.successMessage) return;

    const timeout = window.setTimeout(() => {
      appState.successMessage = null;
    }, 3200);

    return () => window.clearTimeout(timeout);
  });

  function dismiss() {
    appState.successMessage = null;
  }
</script>

{#if appState.successMessage}
  <div class="success-toast" role="status" aria-live="polite">
    <span class="toast-dot"></span>
    <span class="toast-message">{appState.successMessage}</span>
    <button class="toast-dismiss" onclick={dismiss} aria-label="Dismiss success message">x</button>
  </div>
{/if}

<style>
  .success-toast {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 90;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: min(360px, calc(100vw - 32px));
    padding: 10px 12px;
    background: #10251a;
    border: 1px solid #1f7a45;
    border-radius: 8px;
    color: #d8f3df;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .toast-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #22c55e;
    flex-shrink: 0;
  }

  .toast-message {
    min-width: 0;
    font-size: 13px;
    line-height: 1.4;
  }

  .toast-dismiss {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    margin-left: 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: #9dd8ad;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
  }

  .toast-dismiss:hover {
    border-color: #1f7a45;
    background: #153321;
    color: #d8f3df;
  }
</style>
