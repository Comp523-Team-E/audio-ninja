<script lang="ts">
  import { untrack } from 'svelte';
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import WaveSurfer from 'wavesurfer.js';
  import { appState } from '$lib/state.svelte';
  import {
    kindLabel,
    formatMs,
    formatMsDisplay,
    ZOOM_DEFAULT,
    maxZoomForDuration,
    minZoomForDuration,
    shouldHandleWheelZoom,
    zoomFromWheelDelta,
  } from '$lib/utils';
  import { validationProblemMarkerIds } from '$lib/validation';
  import { moveEditingMarkerToMs, setZoomLevel, zoomIn, zoomOut } from '$lib/actions';

  let waveformEl     = $state<HTMLDivElement | null>(null);
  let waveformWrapEl = $state<HTMLDivElement | null>(null);
  let waveformResizeQueued = false;
  let lastWaveformWidth = 0;
  let lastWaveformHeight = 0;
  // The zoom level that WaveSurfer's canvas was last rendered at. While the
  // user is mid-gesture (wheel/pinch zoom), we defer the (expensive) WaveSurfer
  // re-render and instead CSS-scale the rendered canvas to match the current
  // zoom — this keeps zooming smooth even at high zoom levels.
  let lastRenderedZoom = $state(1);
  let pendingZoomRenderTimer: ReturnType<typeof setTimeout> | null = null;
  // While true, queueWaveformResize() will defer the WaveSurfer width update.
  // CSS transform on .waveform-inner provides the visual zoom in the meantime.
  let zoomRenderDeferred = false;
  // ms of inactivity after the last zoom change before we trigger a re-render.
  const ZOOM_RENDER_DEBOUNCE_MS = 120;
  // If the displayed zoom drifts too far from the rendered zoom, force a
  // re-render even mid-gesture so the waveform doesn't get unusably blurry.
  const ZOOM_RENDER_MAX_RATIO = 2.5;
  const validationProblemIds = $derived(validationProblemMarkerIds(appState.markers, appState.validationError));

  // ── Timeline tick helpers ──────────────────────────────────────────────
  const TIMELINE_HEIGHT = 24; // px — keep in sync with CSS

  // Nice major intervals in ms and their minor subdivision counts
  const NICE_INTERVALS_MS = [100, 250, 500, 1000, 2000, 5000, 10000, 15000, 30000, 60000, 120000, 300000, 600000, 900000, 1800000, 3600000];
  const MINOR_COUNTS      = [2,   5,   5,   4,    4,    5,    5,     3,     6,     4,     4,      5,      4,      3,      6,       4     ];

  function formatTickTime(ms: number, durationMs: number): string {
    const totalS = Math.round(ms / 1000);
    const h = Math.floor(totalS / 3600);
    const m = Math.floor((totalS % 3600) / 60);
    const s = totalS % 60;
    if (durationMs >= 3_600_000) {
      return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    }
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function computeTicks(durationMs: number, zoomLevel: number) {
    if (durationMs <= 0) return { major: [] as { pct: number; label: string }[], minor: [] as { pct: number }[] };
    const targetMs = durationMs / (10 * zoomLevel);
    let majorMs   = NICE_INTERVALS_MS[NICE_INTERVALS_MS.length - 1];
    let minorCount = MINOR_COUNTS[MINOR_COUNTS.length - 1];
    for (let i = 0; i < NICE_INTERVALS_MS.length; i++) {
      if (NICE_INTERVALS_MS[i] >= targetMs) { majorMs = NICE_INTERVALS_MS[i]; minorCount = MINOR_COUNTS[i]; break; }
    }
    const minorMs = majorMs / minorCount;
    const major: { pct: number; label: string }[] = [];
    for (let i = 0; i <= Math.ceil(durationMs / majorMs); i++) {
      const t = i * majorMs; if (t > durationMs) break;
      major.push({ pct: t / durationMs * 100, label: formatTickTime(t, durationMs) });
    }
    const minor: { pct: number }[] = [];
    for (let i = 0; i <= Math.ceil(durationMs / minorMs); i++) {
      if (i % minorCount === 0) continue; // coincides with a major tick
      const t = i * minorMs; if (t > durationMs) break;
      minor.push({ pct: t / durationMs * 100 });
    }
    return { major, minor };
  }

  const ticks = $derived(computeTicks(appState.durationMs, appState.zoomLevel));
  const minZoomForFile = $derived(minZoomForDuration(appState.durationMs));
  const maxZoomForFile = $derived(maxZoomForDuration(appState.durationMs));
  const visibleWindowMs = $derived(appState.zoomLevel > 0 && appState.durationMs > 0
    ? appState.durationMs / appState.zoomLevel
    : 0);
  const zoomWindowLabel = $derived(visibleWindowMs > 0 ? formatMsDisplay(visibleWindowMs, visibleWindowMs) : '0.000');

  // Expose the wrap element so external code can scroll it if needed
  $effect(() => { appState.waveformWrapEl = waveformWrapEl; });

  function queueWaveformResize() {
    if (waveformResizeQueued) return;
    waveformResizeQueued = true;
    requestAnimationFrame(() => {
      waveformResizeQueued = false;
      if (!appState.wavesurfer || !waveformEl || !appState.metadata) return;
      // While a continuous zoom gesture is in flight, skip the expensive
      // WaveSurfer re-render — the CSS transform on .waveform-inner handles
      // the visual update. We'll re-render once the gesture settles.
      if (zoomRenderDeferred) return;
      const width = Math.round(waveformEl.clientWidth);
      const height = Math.round((waveformWrapEl?.clientHeight || 0) - TIMELINE_HEIGHT);
      const options: { width?: number; height?: number } = {};
      if (width > 0 && width !== lastWaveformWidth) {
        lastWaveformWidth = width;
        options.width = width;
      }
      if (height > 0 && height !== lastWaveformHeight) {
        lastWaveformHeight = height;
        options.height = height;
      }
      if (options.width || options.height) {
        appState.wavesurfer.setOptions(options);
        lastRenderedZoom = appState.zoomLevel;
      }
    });
  }

  // Force a WaveSurfer re-render at the current zoom level, regardless of
  // whether a zoom gesture is still in flight.
  function flushWaveformResize() {
    if (pendingZoomRenderTimer) {
      clearTimeout(pendingZoomRenderTimer);
      pendingZoomRenderTimer = null;
    }
    zoomRenderDeferred = false;
    if (!appState.wavesurfer || !waveformEl || !appState.metadata) return;
    const width = Math.round(waveformEl.clientWidth);
    const height = Math.round((waveformWrapEl?.clientHeight || 0) - TIMELINE_HEIGHT);
    const options: { width?: number; height?: number } = {};
    if (width > 0 && width !== lastWaveformWidth) {
      lastWaveformWidth = width;
      options.width = width;
    }
    if (height > 0 && height !== lastWaveformHeight) {
      lastWaveformHeight = height;
      options.height = height;
    }
    if (options.width || options.height) {
      appState.wavesurfer.setOptions(options);
    }
    lastRenderedZoom = appState.zoomLevel;
  }

  function scheduleDeferredZoomRender() {
    if (pendingZoomRenderTimer) clearTimeout(pendingZoomRenderTimer);
    pendingZoomRenderTimer = setTimeout(() => {
      pendingZoomRenderTimer = null;
      flushWaveformResize();
    }, ZOOM_RENDER_DEBOUNCE_MS);
  }

  // Keep the waveform canvas width in sync with zoom width changes so
  // the WaveSurfer render does not lag behind the timeline/markers.
  // During continuous zoom gestures we debounce the (expensive) WaveSurfer
  // re-render; the canvas is CSS-scaled in the meantime via the
  // `--waveform-zoom-scale` custom property.
  $effect(() => {
    const zoomLevel = appState.zoomLevel;
    const ws = appState.wavesurfer;
    if (!ws || !waveformEl || !appState.metadata) return;
    if (!zoomRenderDeferred) {
      // Not currently in a deferred-render state. Re-render immediately.
      queueWaveformResize();
      return;
    }
    // Mid-gesture. If zoom drifts too far from the rendered baseline, the
    // CSS-scaled waveform becomes unacceptably blurry/squashed — flush early.
    const ratio = zoomLevel / lastRenderedZoom;
    if (ratio >= ZOOM_RENDER_MAX_RATIO || ratio <= 1 / ZOOM_RENDER_MAX_RATIO) {
      flushWaveformResize();
    } else {
      scheduleDeferredZoomRender();
    }
  });

  // CSS scale factor applied to the WaveSurfer canvas while the renderer's
  // baseline zoom is stale. This makes continuous zoom gestures feel
  // instantaneous without forcing a redraw on every event.
  const waveformZoomScale = $derived(
    lastRenderedZoom > 0 ? appState.zoomLevel / lastRenderedZoom : 1,
  );

  // Initialize (or reinitialize) WaveSurfer whenever the loaded file changes.
  // The cleanup function runs on component destroy or before re-running.
  $effect(() => {
    const filePath = appState.metadata?.filePath;
    if (!filePath || !waveformEl) return;

    // Reset zoom and scroll when a new file is loaded
    appState.zoomLevel = Math.min(
      Math.max(ZOOM_DEFAULT, minZoomForDuration(appState.durationMs)),
      maxZoomForDuration(appState.durationMs),
    );
    if (waveformWrapEl) waveformWrapEl.scrollLeft = 0;

    const ws = WaveSurfer.create({
      container: waveformEl,
      waveColor: '#1d4ed8',
      progressColor: '#3b82f6',
      cursorColor: '#ffffff',
      cursorWidth: 2,
      height: (waveformWrapEl?.clientHeight || 180) - TIMELINE_HEIGHT,
      barWidth: 2,
      barGap: 1,
      barRadius: 2,
      normalize: true,
      interact: false, // pointer events handled manually for real-time drag
    });
    ws.load(convertFileSrc(filePath));
    appState.wavesurfer = ws;
    lastWaveformWidth = 0;
    lastWaveformHeight = 0;
    // Read zoom via untrack so this $effect doesn't depend on zoomLevel —
    // otherwise every zoom change would tear down and recreate WaveSurfer
    // and reset the zoom back to default.
    lastRenderedZoom = untrack(() => appState.zoomLevel);
    zoomRenderDeferred = false;
    if (pendingZoomRenderTimer) {
      clearTimeout(pendingZoomRenderTimer);
      pendingZoomRenderTimer = null;
    }
    queueWaveformResize();

    const resizeObserver = new ResizeObserver(() => {
      queueWaveformResize();
    });
    if (waveformWrapEl) resizeObserver.observe(waveformWrapEl);

    return () => {
      resizeObserver.disconnect();
      if (pendingZoomRenderTimer) {
        clearTimeout(pendingZoomRenderTimer);
        pendingZoomRenderTimer = null;
      }
      ws.destroy();
      appState.wavesurfer = null;
    };
  });

  // Auto-scroll to keep playhead centered during playback
  $effect(() => {
    const pos = appState.positionMs; // reactive dependency
    if (!waveformWrapEl || appState.waveformDragging || appState.zoomLevel <= 1) return;
    if (!appState.followPlayhead) return;
    const pct = appState.durationMs > 0 ? pos / appState.durationMs : 0;
    const total = waveformWrapEl.scrollWidth;
    const visible = waveformWrapEl.clientWidth;
    waveformWrapEl.scrollLeft = Math.max(0, pct * total - visible / 2);
  });

  // ── Zoom controls / interactions ───────────────────────────────────────
  async function handleWheelZoom(e: WheelEvent) {
    if (!waveformWrapEl || !appState.metadata || appState.durationMs <= 0) return;
    if (!shouldHandleWheelZoom({
      deltaX: e.deltaX,
      deltaY: e.deltaY,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
    })) return;
    e.preventDefault();
    // Mark this as a continuous gesture so the zoom $effect debounces the
    // expensive WaveSurfer re-render and just CSS-scales mid-flight.
    zoomRenderDeferred = true;
    const nextZoom = zoomFromWheelDelta(
      appState.zoomLevel,
      e.deltaY,
      appState.durationMs,
      e.ctrlKey || e.metaKey
    );
    setZoomLevel(nextZoom);
  }

  function handleZoomInClick() {
    // Discrete zoom: render immediately rather than waiting for a debounce.
    zoomRenderDeferred = false;
    zoomIn();
    flushWaveformResize();
  }

  function handleZoomOutClick() {
    zoomRenderDeferred = false;
    zoomOut();
    flushWaveformResize();
  }

  // ── Waveform drag seeking ──────────────────────────────────────────────
  // We own pointer events on the container so the playhead moves in real-time.
  //
  // editDragging tracks a pointer-down that started while a marker was being
  // edited.  It is intentionally separate from appState.waveformDragging so
  // that confirmEditMode() (triggered by Enter) cannot clear it: if the user
  // presses Enter while still holding the mouse, the subsequent pointer-up must
  // still be handled as an edit drag — not fall through to the seek path and
  // snap the playhead to the mouse position.

  let waveformWasPlaying = false;
  let editDragging = false;

  function waveformPosFromEvent(e: PointerEvent): number {
    if (!waveformEl) return 0;
    // getBoundingClientRect returns screen coords accounting for scroll offset,
    // so this calculation is correct even when the waveform is zoomed and scrolled.
    const rect = waveformEl.getBoundingClientRect();
    const pct  = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    return pct * appState.durationMs;
  }

  function handlePointerDown(e: PointerEvent) {
    if (!appState.metadata || appState.durationMs === 0) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const ms = waveformPosFromEvent(e);
    if (appState.editingMarkerId) {
      editDragging = true;
      appState.waveformDragging = true;
      moveEditingMarkerToMs(ms);
      return;
    }
    appState.waveformDragging = true;
    waveformWasPlaying = appState.isPlaying;
    if (appState.isPlaying) {
      appState.isPlaying = false;
      invoke('pause').catch(() => {});
    }
    appState.positionMs = ms;
    if (appState.wavesurfer) appState.wavesurfer.setTime(ms / 1000);
  }

  function handlePointerMove(e: PointerEvent) {
    const ms = waveformPosFromEvent(e);
    if (editDragging) {
      moveEditingMarkerToMs(ms);
      return;
    }
    if (!appState.waveformDragging) return;
    appState.positionMs = ms;
    if (appState.wavesurfer) appState.wavesurfer.setTime(ms / 1000);
  }

  async function handlePointerUp(e: PointerEvent) {
    const ms = waveformPosFromEvent(e);
    if (editDragging) {
      editDragging = false;
      appState.waveformDragging = false;
      moveEditingMarkerToMs(ms);
      return;
    }
    if (!appState.waveformDragging) return;
    appState.waveformDragging = false;
    appState.positionMs    = ms;
    appState.syncPositionMs = ms;
    appState.syncWallTime   = performance.now();
    if (appState.wavesurfer) appState.wavesurfer.setTime(ms / 1000);
    try {
      await invoke('seek', { positionMs: Math.round(ms) });
      if (waveformWasPlaying) {
        await invoke('play');
        appState.isPlaying = true;
      }
    } catch (err) {
      appState.error = String(err);
    }
  }
</script>

<div class="zoom-controls">
  <button
    class="zoom-btn"
    onclick={handleZoomOutClick}
    disabled={!appState.metadata || appState.zoomLevel <= minZoomForFile}
    title="Zoom out"
  >−</button>
  <span class="zoom-label">{zoomWindowLabel}s</span>
  <button
    class="zoom-btn"
    onclick={handleZoomInClick}
    disabled={!appState.metadata || appState.zoomLevel >= maxZoomForFile}
    title="Zoom in"
  >+</button>
</div>

<div
  class="waveform-wrap"
  class:waveform-editing={!!appState.editingMarkerId}
  bind:this={waveformWrapEl}
  role="slider"
  aria-label="Playback position"
  aria-valuemin={0}
  aria-valuemax={appState.durationMs}
  aria-valuenow={appState.positionMs}
  tabindex="0"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerUp}
  onwheel={handleWheelZoom}
>
  {#if appState.editingMarkerId}
    <div class="edit-mode-hint">Click to move marker · Enter to confirm · Esc to cancel</div>
  {/if}
  <div class="waveform-scroll" style="width: {appState.zoomLevel * 100}%">
    <div
      class="waveform-inner"
      class:waveform-inner-scaled={waveformZoomScale !== 1}
      style:--waveform-zoom-scale={waveformZoomScale}
      bind:this={waveformEl}
    ></div>
    <!-- Marker overlays — left: pct% is relative to waveform-scroll, same width as the waveform -->
    {#each appState.markers as m (m.id)}
      {@const isEditing = appState.editingMarkerId === m.id}
      {@const draftPct  = appState.durationMs > 0 ? (appState.editingPositionMs / appState.durationMs) * 100 : 0}
      {@const origPct   = appState.durationMs > 0 ? (m.position / appState.durationMs) * 100 : 0}
      {#if isEditing}
        <!-- Ghost at original position -->
        <div
          class="marker-line marker-ghost"
          class:marker-validation-error={validationProblemIds.has(m.id)}
          style="left: {origPct}%"
          title="Original — {formatMs(m.position)}"
        ></div>
        <!-- Draft at new position -->
        <div
          class="marker-line marker-editing"
          class:marker-validation-error={validationProblemIds.has(m.id)}
          style="left: {draftPct}%"
          title="{kindLabel(m.kind)} — {formatMs(appState.editingPositionMs)}"
        ></div>
      {:else}
        <div
          class="marker-line"
          class:marker-start={m.kind === 'start'}
          class:marker-end={m.kind === 'end'}
          class:marker-both={m.kind === 'startEnd'}
          class:marker-selected={appState.selectedMarkerId === m.id}
          class:marker-validation-error={validationProblemIds.has(m.id)}
          style="left: {origPct}%"
          title="{kindLabel(m.kind)} — {formatMs(m.position)}"
        ></div>
      {/if}
    {/each}
    {#if appState.durationMs > 0}
      <div class="timeline-ruler">
        {#each ticks.minor as tick (tick.pct)}
          <div class="tick-minor" style="left: {tick.pct}%"></div>
        {/each}
        {#each ticks.major as tick (tick.pct)}
          <div class="tick-major" style="left: {tick.pct}%">
            <span class="tick-label" class:tick-label-right={tick.pct > 92}>{tick.label}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .zoom-controls {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    background: #0f1419;
    border-bottom: 1px solid #21262d;
  }

  .zoom-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: #1e2a3a;
    border: 1px solid #30363d;
    border-radius: 5px;
    color: #8b949e;
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }

  .zoom-btn:hover:not(:disabled) { color: #e2e8f0; border-color: #4d6a8a; }
  .zoom-btn:disabled { opacity: 0.35; cursor: default; }

  .zoom-label {
    font-size: 11px;
    color: #8b949e;
    min-width: 28px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .waveform-wrap {
    position: relative;
    width: 100%;
    height: clamp(120px, 24vh, 180px);
    flex-shrink: 0;
    background: #0d1117;
    border-bottom: 1px solid #21262d;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .waveform-wrap.waveform-editing {
    cursor: crosshair;
    outline: 2px solid #f97316;
    outline-offset: -2px;
  }

  .edit-mode-hint {
    position: absolute;
    top: 6px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    background: rgba(249, 115, 22, 0.85);
    color: #fff;
    font-size: 11px;
    padding: 2px 10px;
    border-radius: 4px;
    pointer-events: none;
    white-space: nowrap;
    user-select: none;
  }

  .waveform-scroll {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 100%;
  }

  .waveform-inner {
    width: 100%;
    flex: 1;
    min-height: 0;
  }

  /* During a continuous zoom gesture we defer the (expensive) WaveSurfer
     re-render and instead CSS-scale the existing canvas so the waveform stays
     aligned with the timeline & markers. The transform is horizontal-only with
     a left origin so playhead/marker positions remain meaningful. */
  .waveform-inner-scaled {
    transform: scaleX(var(--waveform-zoom-scale, 1));
    transform-origin: 0 0;
    will-change: transform;
  }

  .timeline-ruler {
    position: relative;
    width: 100%;
    height: 24px; /* = TIMELINE_HEIGHT */
    flex-shrink: 0;
    background: #080c11;
    border-top: 1px solid #21262d;
    overflow: hidden;
    pointer-events: none;
  }

  .tick-major {
    position: absolute;
    top: 0;
    width: 1px;
    height: 100%;
    background: #2e3d52;
    transform: translateX(-50%);
  }

  .tick-label {
    position: absolute;
    left: 3px;
    top: 4px;
    font-size: 9px;
    color: #5c7080;
    white-space: nowrap;
    user-select: none;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .tick-label-right {
    left: auto;
    right: 3px;
  }

  .tick-minor {
    position: absolute;
    top: 0;
    width: 1px;
    height: 7px;
    background: #1e2a38;
    transform: translateX(-50%);
  }

  .marker-line {
    position: absolute;
    top: 0;
    bottom: 24px; /* stop above timeline ruler (= TIMELINE_HEIGHT) */
    width: 2px;
    background: #22c55e;
    pointer-events: none;
    transform: translateX(-50%);
    opacity: 0.9;
    z-index: 10;
    transition: opacity 0.1s;
  }

  .marker-line.marker-start {
    background: #22c55e;
  }

  .marker-line.marker-end {
    background: #f87171;
  }

  .marker-line.marker-both {
    background: linear-gradient(to bottom, #22c55e 0 50%, #f87171 50% 100%);
  }

  .marker-line.marker-selected {
    opacity: 1;
  }

  .marker-line.marker-ghost {
    background: #f97316;
    opacity: 0.3;
  }

  .marker-line.marker-editing {
    background: #f97316;
    opacity: 1;
    animation: editing-pulse 1s ease-in-out infinite alternate;
  }

  .marker-line.marker-validation-error {
    background: #f87171;
    opacity: 1;
    width: 3px;
    box-shadow: 0 0 8px rgba(248, 113, 113, 0.85);
  }

  @keyframes editing-pulse {
    from { opacity: 0.6; }
    to   { opacity: 1; }
  }
</style>
