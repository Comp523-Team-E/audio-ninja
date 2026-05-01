<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { appState } from '$lib/state.svelte';
  import {
    stopPolling, stopRaf, handleKeydown, loadShortcuts,
    openFile, openFileFromPath, isSupportedMediaPath,
    importCsv, togglePlay, setSpeed, handleLoop,
    addMarkerNoKind, addMarkerAt, deleteMarker, splitStartEndMarker,
    renameSegment, exportAudioSegments,
    stepBack, stepFwd, handleFollowPlayhead
  } from '$lib/actions';

  import WelcomeScreen from '../components/WelcomeScreen.svelte';
  import Header from '../components/Header.svelte';
  import WaveformDisplay from '../components/WaveformDisplay.svelte';
  import PlaybackControls from '../components/PlaybackControls.svelte';
  import MarkerPanel from '../components/MarkerPanel.svelte';
  import SegmentPanel from '../components/SegmentPanel.svelte';
  import ShortcutsPanel from '../components/ShortcutsPanel.svelte';
  import ErrorAlert from '../components/ErrorAlert.svelte';
  import SuccessToast from '../components/SuccessToast.svelte';

  let shortcutsCollapsed = $state(false);
  // Drag-and-drop hover state — we listen to the OS-level event from the
  // Tauri webview (not HTML5 dragover/drop, which don't deliver real paths).
  let dragHover     = $state(false);
  let dragSupported = $state(true);

  onMount(() => {
    loadShortcuts();
    document.addEventListener('keydown', handleKeydown);
    const media = window.matchMedia('(max-width: 900px)');
    const syncShortcutsLayout = () => {
      shortcutsCollapsed = media.matches;
    };
    syncShortcutsLayout();
    media.addEventListener('change', syncShortcutsLayout);

    // Tauri's webview emits OS-level drag-drop events with absolute paths.
    // Only the first dropped file is opened; extras are ignored.
    let unlistenDragDrop: (() => void) | null = null;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === 'enter') {
          dragSupported = p.paths.some(isSupportedMediaPath);
          dragHover = true;
        } else if (p.type === 'leave') {
          dragHover = false;
        } else if (p.type === 'drop') {
          dragHover = false;
          const target = p.paths.find(isSupportedMediaPath) ?? p.paths[0];
          if (target) openFileFromPath(target);
        }
      })
      .then((unlisten) => { unlistenDragDrop = unlisten; })
      .catch(() => {
        // Listener unavailable (e.g. running outside Tauri in unit tests) —
        // drag-and-drop simply won't be wired up.
      });

    return () => {
      document.removeEventListener('keydown', handleKeydown);
      media.removeEventListener('change', syncShortcutsLayout);
      if (unlistenDragDrop) unlistenDragDrop();
    };
  });

  onDestroy(() => {
    stopPolling();
    stopRaf();
    if (appState.wavesurfer) appState.wavesurfer.destroy();
  });
</script>

<ErrorAlert />
<SuccessToast />

{#if dragHover}
  <div class="drop-overlay" class:drop-overlay-invalid={!dragSupported}>
    <div class="drop-overlay-card">
      {#if dragSupported}
        <p class="drop-overlay-title">Drop to open</p>
        <p class="drop-overlay-sub">{appState.metadata ? 'Replaces the currently loaded file' : 'MP4, MP3, WAV, FLAC, OGG, AAC, M4A'}</p>
      {:else}
        <p class="drop-overlay-title">Unsupported file type</p>
        <p class="drop-overlay-sub">Supported: MP4, MP3, WAV, FLAC, OGG, AAC, M4A</p>
      {/if}
    </div>
  </div>
{/if}

{#if !appState.metadata}
  <WelcomeScreen onOpenFile={openFile} />
{:else}
  <div class="app">
    <Header onOpenFile={openFile}
            onImportCsv={importCsv}
            onExportAudioSegments={exportAudioSegments}
    />
    <WaveformDisplay />
    <PlaybackControls
      onStepBack={stepBack}
      onTogglePlay={togglePlay}
      onStepFwd={stepFwd}
      onSetSpeed={setSpeed}
      onToggleLoop={handleLoop}
      onToggleFollow={handleFollowPlayhead}
    />
	    <div class="panels" class:shortcuts-collapsed={shortcutsCollapsed}>
	      <MarkerPanel
	        onAddMarkerNoKind={addMarkerNoKind}
	        onDeleteMarker={deleteMarker}
	        onAddMarkerAt={addMarkerAt}
	        onSplitStartEndMarker={splitStartEndMarker}
	      />
	      <SegmentPanel onRenameSegment={renameSegment} />
	      <ShortcutsPanel
	        collapsed={shortcutsCollapsed}
	        onToggleCollapsed={() => { shortcutsCollapsed = !shortcutsCollapsed; }}
	      />
	    </div>
  </div>
{/if}

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    background: #0d1117;
    color: #e2e8f0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    font-size: 13px;
    line-height: 1.4;
    overflow: hidden;
    height: 100vh;
    user-select: none;
    -webkit-user-select: none;
  }

  :global(input),
  :global(textarea),
  :global([contenteditable='true']) {
    user-select: text;
    -webkit-user-select: text;
  }

  :global(.copyable-text) {
    user-select: text;
    -webkit-user-select: text;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

	  .panels {
	    display: grid;
	    grid-template-columns: minmax(180px, 1fr) minmax(260px, 2fr) minmax(180px, 1fr);
	    flex: 1;
	    overflow: hidden;
	    min-height: 0;
	  }

	  .panels.shortcuts-collapsed {
	    grid-template-columns: minmax(180px, 1fr) minmax(260px, 2fr) 58px;
	  }

	  @media (max-width: 900px) {
	    .panels {
	      grid-template-columns: minmax(160px, 1fr) minmax(240px, 2fr) minmax(170px, 0.9fr);
	    }

	    .panels.shortcuts-collapsed {
	      grid-template-columns: minmax(160px, 1fr) minmax(240px, 2fr) 58px;
	    }
	  }

  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(13, 17, 23, 0.78);
    border: 3px dashed #3b82f6;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    backdrop-filter: blur(2px);
  }

  .drop-overlay-invalid {
    border-color: #f87171;
    background: rgba(40, 13, 13, 0.78);
  }

  .drop-overlay-card {
    text-align: center;
    color: #e2e8f0;
    padding: 24px 40px;
    border-radius: 12px;
    background: rgba(22, 27, 34, 0.9);
    border: 1px solid #30363d;
  }

  .drop-overlay-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 6px;
  }

  .drop-overlay-invalid .drop-overlay-title {
    color: #f87171;
  }

  .drop-overlay-sub {
    font-size: 12px;
    color: #8b949e;
  }

  :global(::-webkit-scrollbar) { width: 6px; }
  :global(::-webkit-scrollbar-track) { background: transparent; }
  :global(::-webkit-scrollbar-thumb) { background: #30363d; border-radius: 3px; }
  :global(::-webkit-scrollbar-thumb:hover) { background: #4d5b6b; }
</style>
