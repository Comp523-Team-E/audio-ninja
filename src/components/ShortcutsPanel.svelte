<script lang="ts">
  import { appState } from '$lib/state.svelte';
  import {
    ACTION_NAMES,
    DEFAULT_SHORTCUTS,
    formatShortcutKey,
    matchesShortcut,
  } from '$lib/shortcuts';
  import type { ActionName, ShortcutKey } from '$lib/shortcuts';
  import { saveShortcuts } from '$lib/actions';

  let { collapsed = false, onToggleCollapsed }: {
    collapsed?: boolean;
    onToggleCollapsed?: () => void;
  } = $props();

  let editMode = $state(false);

  interface CaptureTarget {
    action: ActionName;
    index: number; // -1 = add new, >=0 = replace existing
  }
  let capturing = $state<CaptureTarget | null>(null);
  let conflictAction = $state<ActionName | null>(null);
  let conflictTimer: ReturnType<typeof setTimeout> | null = null;

  // Document-level listener active only while capturing — fires before the
  // global handleKeydown so we can intercept the keypress.
  $effect(() => {
    if (!capturing) return;

    function onKeydown(e: KeyboardEvent) {
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
      e.preventDefault();
      e.stopPropagation();

      const cap = capturing;
      if (!cap) return;

      const newKey: ShortcutKey = {
        key: e.key,
        ...(e.ctrlKey || e.metaKey ? { ctrl: true } : {}),
        ...(e.shiftKey ? { shift: true } : {}),
        ...(e.altKey ? { alt: true } : {}),
      };

      const conflict = ACTION_NAMES.find(
        a => a !== cap.action && matchesShortcut(e, appState.shortcuts[a])
      );

      if (conflict) {
        conflictAction = conflict;
        if (conflictTimer) clearTimeout(conflictTimer);
        conflictTimer = setTimeout(() => { conflictAction = null; }, 2000);
        capturing = null;
        return;
      }

      const current = appState.shortcuts[cap.action];
      const updated =
        cap.index === -1
          ? [...current, newKey]
          : current.map((k, i) => (i === cap.index ? newKey : k));

      appState.shortcuts = { ...appState.shortcuts, [cap.action]: updated };
      saveShortcuts(appState.shortcuts);
      capturing = null;
    }

    document.addEventListener('keydown', onKeydown, true);
    return () => document.removeEventListener('keydown', onKeydown, true);
  });

  interface ShortcutRow {
    section: string;
    action: ActionName;
    label: string;
  }

  const ROWS: ShortcutRow[] = [
    { section: 'Control Playback', action: 'stepForward',          label: 'Seek forward' },
    { section: 'Control Playback', action: 'stepBackward',         label: 'Seek backward' },
    { section: 'Control Playback', action: 'togglePlay',           label: 'Play/Pause' },
    { section: 'Control Playback', action: 'seekToEnd',            label: 'Seek to end' },
    { section: 'Control Playback', action: 'seekToStart',          label: 'Seek to start' },
    { section: 'Control Playback', action: 'setSpeed1',            label: 'Speed 0.5×' },
    { section: 'Control Playback', action: 'setSpeed2',            label: 'Speed 0.75×' },
    { section: 'Control Playback', action: 'setSpeed3',            label: 'Speed 1×' },
    { section: 'Control Playback', action: 'setSpeed4',            label: 'Speed 1.5×' },
    { section: 'Control Playback', action: 'setSpeed5',            label: 'Speed 2×' },
    { section: 'Waveform Zoom',    action: 'zoomOut',              label: 'Zoom out' },
    { section: 'Waveform Zoom',    action: 'zoomIn',               label: 'Zoom in' },
    { section: 'Waveform Zoom',    action: 'toggleFollowPlayhead', label: 'Follow playhead' },
    { section: 'Add Marker',       action: 'addStartMarker',       label: 'Add start' },
    { section: 'Add Marker',       action: 'addEndMarker',         label: 'Add end' },
    { section: 'Add Marker',       action: 'addStartEndMarker',    label: 'Add start+end' },
    { section: 'Manage Markers',   action: 'seekToPrevMarker',     label: 'Seek to previous' },
    { section: 'Manage Markers',   action: 'seekToNextMarker',     label: 'Seek to next' },
    { section: 'Manage Markers',   action: 'splitStartEndMarker',  label: 'Split start+end' },
    { section: 'Manage Markers',   action: 'deleteMarker',         label: 'Delete selected' },
    { section: 'Edit Marker',      action: 'nudgeLeft',            label: 'Nudge left 100ms' },
    { section: 'Edit Marker',      action: 'nudgeRight',           label: 'Nudge right 100ms' },
    { section: 'Edit Marker',      action: 'confirmEdit',          label: 'Confirm position' },
    { section: 'Edit Marker',      action: 'cancelEdit',           label: 'Cancel editing' },
    { section: 'Export',           action: 'exportCsv',            label: 'Export CSV' },
  ];

  const SECTIONS = [...new Set(ROWS.map(r => r.section))];
  function rowsForSection(s: string) { return ROWS.filter(r => r.section === s); }

  function startCapture(action: ActionName, index: number) {
    capturing = { action, index };
    conflictAction = null;
    if (conflictTimer) { clearTimeout(conflictTimer); conflictTimer = null; }
  }

  function removeBinding(action: ActionName, index: number) {
    const current = appState.shortcuts[action];
    if (current.length <= 1) return;
    appState.shortcuts = { ...appState.shortcuts, [action]: current.filter((_, i) => i !== index) };
    saveShortcuts(appState.shortcuts);
  }

  function resetToDefaults() {
    appState.shortcuts = { ...DEFAULT_SHORTCUTS };
    saveShortcuts(appState.shortcuts);
    capturing = null;
    conflictAction = null;
  }

  function toggleEditMode() {
    editMode = !editMode;
    capturing = null;
    conflictAction = null;
  }

  function isCapturing(action: ActionName, index: number) {
    return capturing?.action === action && capturing?.index === index;
  }

  const isDefaultConfig = $derived(
    JSON.stringify(appState.shortcuts) === JSON.stringify(DEFAULT_SHORTCUTS)
  );
</script>

<div class="panel shortcuts-panel" class:shortcuts-collapsed={collapsed}>
  <div class="panel-header">
    <h3 class="panel-title">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
        <rect x="2" y="4" width="20" height="16" rx="2"/>
        <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M6 12h.01M18 12h.01M10 12h4M6 16h12"/>
      </svg>
      {#if !collapsed}
        Keyboard Shortcuts
      {/if}
    </h3>
    <div class="header-actions">
      {#if !collapsed}
        <button
          class="edit-btn"
          class:active={editMode}
          onclick={toggleEditMode}
          title={editMode ? 'Done editing' : 'Edit shortcuts'}
        >
          {editMode ? 'Done' : 'Edit'}
        </button>
      {/if}
      <button
        class="collapse-btn"
        onclick={() => { if (editMode) toggleEditMode(); onToggleCollapsed?.(); }}
        title={collapsed ? 'Show keyboard shortcuts' : 'Hide keyboard shortcuts'}
        aria-label={collapsed ? 'Show keyboard shortcuts' : 'Hide keyboard shortcuts'}
      >
        {collapsed ? '‹' : '›'}
      </button>
    </div>
  </div>

  {#if !collapsed}
    <div class="shortcut-list">
      {#if conflictAction}
        <div class="conflict-banner">
          Already bound to <strong>{ROWS.find(r => r.action === conflictAction)?.label ?? conflictAction}</strong> — choose a different key
        </div>
      {/if}

      {#each SECTIONS as section}
        <div class="shortcut-subheading">{section}</div>

        <!-- Special combined row for seek step (view mode only) -->
        {#if section === 'Control Playback' && !editMode}
          <div class="shortcut-row">
            <div class="key-group">
              <kbd>{formatShortcutKey(appState.shortcuts.stepBackward[0])}/{formatShortcutKey(appState.shortcuts.stepForward[0])}</kbd>
            </div>
            <span class="seek-step-label">
              Seek ±
              <input class="step-input" type="number" min="100" max="60000" bind:value={appState.stepMs} />
              ms
            </span>
          </div>
        {/if}

        {#each rowsForSection(section).filter(r =>
          editMode || (r.action !== 'stepForward' && r.action !== 'stepBackward')
        ) as row}
          <div class="shortcut-row" class:conflict-row={conflictAction === row.action}>
            <div class="key-group">
              {#if editMode}
                {#each appState.shortcuts[row.action] as key, i}
                  {#if i > 0}<span class="or-sep">or</span>{/if}
                  <div class="key-item">
                    <button
                      class="capture-btn"
                      class:capturing={isCapturing(row.action, i)}
                      class:conflict={conflictAction === row.action}
                      onclick={() => startCapture(row.action, i)}
                    >
                      {isCapturing(row.action, i) ? 'Press a key…' : formatShortcutKey(key)}
                    </button>
                    {#if appState.shortcuts[row.action].length > 1}
                      <button
                        class="remove-btn"
                        onclick={() => removeBinding(row.action, i)}
                        title="Remove this binding"
                        aria-label="Remove binding"
                      >×</button>
                    {/if}
                  </div>
                {/each}
                <button
                  class="add-btn"
                  class:capturing={isCapturing(row.action, -1)}
                  onclick={() => startCapture(row.action, -1)}
                  title="Add another key binding"
                >
                  {isCapturing(row.action, -1) ? 'Press a key…' : '+'}
                </button>
              {:else}
                {#each appState.shortcuts[row.action] as key, i}
                  {#if i > 0}<span class="or-sep">or</span>{/if}
                  <kbd>{formatShortcutKey(key)}</kbd>
                {/each}
              {/if}
            </div>

            {#if row.action === 'stepForward' && editMode}
              <span class="seek-step-label">
                Seek forward ±
                <input class="step-input" type="number" min="100" max="60000" bind:value={appState.stepMs} />
                ms
              </span>
            {:else}
              <span>{row.label}</span>
            {/if}
          </div>
        {/each}
      {/each}

      {#if editMode}
        <button class="reset-btn" class:modified={!isDefaultConfig} onclick={resetToDefaults}>Reset to Defaults</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }

  .shortcuts-panel {
    background: #0f1419;
  }

  .shortcuts-collapsed {
    align-items: stretch;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px 8px;
    border-bottom: 1px solid #21262d;
    flex-shrink: 0;
  }

  .shortcuts-collapsed .panel-header {
    justify-content: center;
    padding: 10px 6px 8px;
    gap: 8px;
  }

  .panel-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #c9d1d9;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .edit-btn {
    padding: 2px 8px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 5px;
    color: #8b949e;
    cursor: pointer;
    font-size: 11px;
    line-height: 1.6;
  }

  .edit-btn:hover, .edit-btn.active {
    color: #e2e8f0;
    border-color: #2563eb;
  }

  .collapse-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex: 0 0 24px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #8b949e;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
  }

  .collapse-btn:hover {
    color: #e2e8f0;
    border-color: #4d6a8a;
  }

  .shortcut-list {
    padding: 8px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: auto;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 26px;
    min-width: max-content;
    flex-wrap: nowrap;
  }

  .key-group {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .key-item {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .shortcut-row kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 36px;
    padding: 4px 7px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 6px;
    font-size: 11px;
    color: #c9d1d9;
    font-family: inherit;
    white-space: nowrap;
  }

  .shortcut-row span {
    font-size: 12px;
    color: #8b949e;
    text-align: right;
    white-space: nowrap;
  }

  .or-sep {
    font-size: 10px;
    color: #4b5563;
    text-align: left;
    flex-shrink: 0;
  }

  .shortcut-subheading {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #4b5563;
    margin-top: 10px;
    margin-bottom: 2px;
  }

  .seek-step-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #8b949e;
  }

  .step-input {
    width: 64px;
    padding: 4px 6px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 5px;
    color: #c9d1d9;
    font-size: 11px;
    outline: none;
    text-align: right;
  }

  .step-input:focus { border-color: #2563eb; }

  .capture-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 36px;
    padding: 4px 7px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 6px;
    font-size: 11px;
    color: #c9d1d9;
    font-family: inherit;
    white-space: nowrap;
    cursor: pointer;
  }

  .capture-btn:hover {
    border-color: #2563eb;
    color: #e2e8f0;
  }

  /* Border-color animation — no opacity changes so nothing bleeds through */
  .capture-btn.capturing {
    background: #1e3a5f;
    color: #93c5fd;
    animation: pulse-border 1s ease-in-out infinite;
  }

  .capture-btn.conflict {
    border-color: #dc2626;
    background: #3a1e1e;
    color: #fca5a5;
  }

  .remove-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    background: #261919;
    border: none;
    border-radius: 3px;
    color: #7a3e3e;
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
  }

  .remove-btn:hover {
    background: #3a1e1e;
    color: #fca5a5;
  }

  .add-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 26px;
    padding: 0 6px;
    background: transparent;
    border: 1px dashed #30363d;
    border-radius: 6px;
    color: #4b5563;
    cursor: pointer;
    font-size: 14px;
    font-family: inherit;
    white-space: nowrap;
  }

  .add-btn:hover {
    border-color: #2563eb;
    color: #93c5fd;
    background: #1e3a5f;
  }

  .add-btn.capturing {
    border-style: solid;
    border-color: #2563eb;
    background: #1e3a5f;
    color: #93c5fd;
    font-size: 11px;
    animation: pulse-border 1s ease-in-out infinite;
  }

  .conflict-banner {
    font-size: 11px;
    color: #fca5a5;
    background: #3a1e1e;
    border: 1px solid #dc2626;
    border-radius: 5px;
    padding: 6px 10px;
    margin-bottom: 4px;
  }

  .reset-btn {
    margin-top: 10px;
    padding: 5px 12px;
    background: #1e2a3a;
    border: 1px solid #21262d;
    border-radius: 5px;
    color: #4b5563;
    cursor: pointer;
    font-size: 11px;
    align-self: flex-start;
  }

  .reset-btn.modified {
    color: #e2e8f0;
    border-color: #4d6a8a;
  }

  .reset-btn.modified:hover {
    color: #fff;
    border-color: #60a5fa;
  }

  @keyframes pulse-border {
    0%, 100% { border-color: #2563eb; }
    50%       { border-color: #60a5fa; }
  }
</style>
