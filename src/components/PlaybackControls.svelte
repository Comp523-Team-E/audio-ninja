<script lang="ts">
  import { appState } from '$lib/state.svelte';
  import { formatMs, SPEEDS } from '$lib/utils';

  let { onStepBack, onTogglePlay, onStepFwd, onSetSpeed, onToggleLoop }: {
    onStepBack: () => Promise<void>;
    onTogglePlay: () => Promise<void>;
    onStepFwd: () => Promise<void>;
    onSetSpeed: (s: number) => Promise<void>;
    onToggleLoop: (enabled: boolean) => Promise<void>;
  } = $props();
</script>

<div class="controls-bar">
  <div class="controls-left">
    <button class="ctrl-btn" onclick={onStepBack} title="Step back">
      <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18"><path d="M6 6h2v12H6zm3.5 6 8.5 6V6z"/></svg>
    </button>
    <button class="play-btn" onclick={onTogglePlay} title={appState.isPlaying ? 'Pause' : 'Play'}>
      {#if appState.isPlaying}
        <svg viewBox="0 0 24 24" fill="currentColor" width="24" height="24"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
      {:else}
        <svg viewBox="0 0 24 24" fill="currentColor" width="24" height="24"><path d="M8 5v14l11-7z"/></svg>
      {/if}
    </button>
    <button class="ctrl-btn" onclick={onStepFwd} title="Step forward">
      <svg viewBox="0 0 24 24" fill="currentColor" width="18" height="18"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
    </button>
  </div>

  <div class="controls-right">
    <div class="speed-group">
      <span class="controls-label">Speed</span>
      {#each SPEEDS as s}
        <button
          class="speed-btn"
          class:speed-active={appState.speed === s}
          onclick={() => onSetSpeed(s)}
        >{s}x</button>
      {/each}
    </div>

    <label class="loop-label">
      <input
        type="checkbox"
        checked={appState.looping}
        onchange={(e) => onToggleLoop((e.currentTarget as HTMLInputElement).checked)}
      />
      Loop
    </label>

    <span class="time-display">
      {formatMs(appState.positionMs)} / {formatMs(appState.durationMs)}
    </span>
  </div>
</div>

<style>
  .controls-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-bottom: 1px solid #21262d;
    background: #0f1419;
    flex-shrink: 0;
  }

  .controls-left {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: #8b949e;
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }

  .ctrl-btn:hover { color: #e2e8f0; background: #1e2a3a; }

  .play-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    background: #2563eb;
    border: none;
    border-radius: 50%;
    color: #fff;
    cursor: pointer;
    transition: background 0.15s, transform 0.1s;
  }

  .play-btn:hover { background: #1d4ed8; }
  .play-btn:active { transform: scale(0.95); }

  .controls-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .controls-label {
    font-size: 11px;
    color: #8b949e;
  }

  .speed-group {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .speed-btn {
    padding: 3px 7px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 5px;
    color: #8b949e;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .speed-btn:hover { color: #e2e8f0; border-color: #4d6a8a; }

  .speed-active {
    background: #2563eb;
    border-color: #2563eb;
    color: #fff !important;
  }

  .loop-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #8b949e;
    cursor: pointer;
    user-select: none;
  }

  .loop-label input { cursor: pointer; accent-color: #2563eb; }

  .time-display {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: #3b82f6;
    font-weight: 600;
    white-space: nowrap;
  }
</style>
