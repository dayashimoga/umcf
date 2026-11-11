<script lang="ts">
  /**
   * Svelte 5 Ultra-Low-Latency WebRTC/QUIC Stream Player, Telemetry HUD & Multi-Modal Input (UMCF S6.P3.SP3)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  export interface StreamTelemetry {
    streamId: number;
    frameSeq: number;
    glassToGlassLatencyMs: number;
    encodeLatencyMs: number;
    networkRttMs: number;
    jitterMs: number;
    packetLossPct: number;
    fecRecoveryPct: number;
    bitrateMbps: number;
    targetBitrateMbps: number;
    resolution: string;
    fps: number;
    abrState: 'RampUp' | 'Hold' | 'Backoff' | 'ResolutionDownshift';
    delayGradientMs: number;
    nackCount: number;
    pliCount: number;
  }

  interface Props {
    streamId?: number;
    hostName?: string;
    codec?: 'H264' | 'HEVC' | 'AV1';
    initialBitrateMbps?: number;
    initialViewOnly?: boolean;
    onKeyframeRequested?: (streamId: number) => void;
    onBitrateLimitChanged?: (limitMbps: number) => void;
    onEmergencyReleaseInputs?: () => void;
  }

  let {
    streamId = 1,
    hostName = 'Gaming-Host-Desktop',
    codec = 'H264',
    initialBitrateMbps = 25,
    initialViewOnly = false,
    onKeyframeRequested,
    onBitrateLimitChanged,
    onEmergencyReleaseInputs,
  }: Props = $props();

  // Reactive state using Svelte 5 runes
  let isConnected = $state<boolean>(true);
  let isHudVisible = $state<boolean>(true);
  let hudMode = $state<'compact' | 'expanded'>('expanded');
  let selectedBitrateCap = $state<number>(initialBitrateMbps);
  let autoAbrEnabled = $state<boolean>(true);
  let isFullscreen = $state<boolean>(false);
  let isViewOnly = $state<boolean>(initialViewOnly);
  let touchMode = $state<'direct' | 'trackpad'>('direct');
  let showVirtualGamepad = $state<boolean>(false);
  let lastReleasedCount = $state<number | null>(null);
  let playerContainerRef = $state<HTMLDivElement | null>(null);

  // Live telemetry state
  let telemetry = $state<StreamTelemetry>({
    streamId,
    frameSeq: 10420,
    glassToGlassLatencyMs: 16.4,
    encodeLatencyMs: 2.1,
    networkRttMs: 8.2,
    jitterMs: 0.8,
    packetLossPct: 1.2,
    fecRecoveryPct: 99.4,
    bitrateMbps: initialBitrateMbps,
    targetBitrateMbps: initialBitrateMbps,
    resolution: '1920 × 1080',
    fps: 60,
    abrState: 'Hold',
    delayGradientMs: 0.35,
    nackCount: 14,
    pliCount: 2,
  });

  // Derived health indicators
  let latencyStatus = $derived<'optimal' | 'warning' | 'critical'>(
    telemetry.glassToGlassLatencyMs < 20.0
      ? 'optimal'
      : telemetry.glassToGlassLatencyMs < 35.0
        ? 'warning'
        : 'critical'
  );

  let abrColor = $derived<string>(
    telemetry.abrState === 'Hold' || telemetry.abrState === 'RampUp'
      ? '#10b981'
      : telemetry.abrState === 'Backoff'
        ? '#f59e0b'
        : '#ef4444'
  );

  // Simulation ticker for realistic telemetry jitter
  $effect(() => {
    if (!isConnected) return;

    const interval = setInterval(() => {
      telemetry.frameSeq += 1;
      const jitterVariation = (Math.random() - 0.5) * 0.4;
      telemetry.jitterMs = Math.max(0.2, Number((telemetry.jitterMs + jitterVariation * 0.1).toFixed(2)));
      
      const rttVariation = (Math.random() - 0.5) * 0.6;
      telemetry.networkRttMs = Math.max(4.0, Number((8.2 + rttVariation).toFixed(1)));
      
      telemetry.glassToGlassLatencyMs = Number(
        (telemetry.encodeLatencyMs + telemetry.networkRttMs + telemetry.jitterMs + 5.2).toFixed(1)
      );

      if (telemetry.abrState === 'Backoff' && autoAbrEnabled) {
        telemetry.bitrateMbps = Math.max(5.0, Number((telemetry.bitrateMbps * 0.95).toFixed(1)));
      } else if (telemetry.abrState === 'RampUp' && autoAbrEnabled) {
        telemetry.bitrateMbps = Math.min(selectedBitrateCap, Number((telemetry.bitrateMbps + 0.2).toFixed(1)));
      }
    }, 100);

    return () => clearInterval(interval);
  });

  function handleRequestKeyframe() {
    telemetry.pliCount += 1;
    if (onKeyframeRequested) {
      onKeyframeRequested(streamId);
    }
  }

  function handleBitrateSlider(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    selectedBitrateCap = val;
    if (onBitrateLimitChanged) {
      onBitrateLimitChanged(val);
    }
  }

  function handleEmergencyRelease() {
    lastReleasedCount = 7;
    if (onEmergencyReleaseInputs) {
      onEmergencyReleaseInputs();
    }
    setTimeout(() => {
      lastReleasedCount = null;
    }, 2500);
  }

  function toggleFullscreen() {
    if (!playerContainerRef) return;
    if (!document.fullscreenElement) {
      playerContainerRef.requestFullscreen().then(() => {
        isFullscreen = true;
      }).catch(console.error);
    } else {
      document.exitFullscreen().then(() => {
        isFullscreen = false;
      }).catch(console.error);
    }
  }
</script>

<div
  bind:this={playerContainerRef}
  class="stream-player-container"
  role="region"
  aria-label="Low-latency stream display and input return surface"
>
  <!-- Video stream viewport with embedded HUD and touch surface -->
  <div class="video-viewport">
    <div class="video-mock-surface">
      <div class="stream-center-badge">
        <span class="host-pill">🖥️ {hostName}</span>
        <span class="codec-pill">{codec} 60 FPS</span>
        <span class="res-pill">{telemetry.resolution}</span>
        {#if isViewOnly}
          <span class="view-only-pill">🔒 View-Only Mode</span>
        {:else}
          <span class="full-control-pill">🎮 Full Control</span>
        {/if}
      </div>

      <!-- Watermark Barcode & Monotonic Timestamp representation -->
      <div class="synthetic-barcode-badge">
        <span class="barcode-matrix">█ ▄ █ ▌ █ █ ▌ ▄</span>
        <span class="timestamp-label">PTS: {(telemetry.frameSeq * 16666).toLocaleString()} µs</span>
      </div>

      <!-- On-Screen Virtual Gamepad Overlay (Touch/Tablet) -->
      {#if showVirtualGamepad && !isViewOnly}
        <div class="virtual-gamepad-overlay" aria-label="On-Screen Gamepad Controls">
          <!-- Left D-Pad -->
          <div class="gamepad-cluster left">
            <div class="dpad-grid">
              <button class="gamepad-btn up" aria-label="D-Pad Up">▲</button>
              <button class="gamepad-btn left" aria-label="D-Pad Left">◀</button>
              <button class="gamepad-btn center"></button>
              <button class="gamepad-btn right" aria-label="D-Pad Right">▶</button>
              <button class="gamepad-btn down" aria-label="D-Pad Down">▼</button>
            </div>
            <span class="stick-label">L-Stick</span>
          </div>

          <!-- Right Face Buttons (A, B, X, Y) -->
          <div class="gamepad-cluster right">
            <div class="face-buttons-grid">
              <button class="gamepad-btn btn-y" aria-label="Button Y">Y</button>
              <button class="gamepad-btn btn-x" aria-label="Button X">X</button>
              <button class="gamepad-btn btn-b" aria-label="Button B">B</button>
              <button class="gamepad-btn btn-a" aria-label="Button A">A</button>
            </div>
            <span class="stick-label">R-Stick</span>
          </div>
        </div>
      {/if}

      <!-- Emergency Release Toast -->
      {#if lastReleasedCount !== null}
        <div class="release-toast" role="alert">
          🛡️ Released {lastReleasedCount} held inputs on host OS (Stuck-Key Watchdog)
        </div>
      {/if}
    </div>

    <!-- Telemetry HUD Overlay -->
    {#if isHudVisible}
      <div
        class="telemetry-hud {hudMode}"
        role="status"
        aria-live="polite"
      >
        <div class="hud-header">
          <div class="hud-title-group">
            <span class="hud-dot" style="background-color: {abrColor}"></span>
            <span class="hud-title">QUIC Transport Telemetry</span>
            <span class="hud-state" style="color: {abrColor}">[{telemetry.abrState}]</span>
          </div>
          <div class="hud-controls-group">
            <button
              class="hud-btn-mini"
              onclick={() => hudMode = hudMode === 'expanded' ? 'compact' : 'expanded'}
              title="Toggle HUD detail level"
            >
              {hudMode === 'expanded' ? 'Compact' : 'Expand'}
            </button>
            <button
              class="hud-btn-mini"
              onclick={() => isHudVisible = false}
              title="Close HUD overlay"
            >
              ✕
            </button>
          </div>
        </div>

        <!-- Metric Grid -->
        <div class="hud-metrics-grid">
          <div class="metric-cell">
            <span class="metric-label">Glass-to-Glass</span>
            <span class="metric-value {latencyStatus}">
              {telemetry.glassToGlassLatencyMs} ms
            </span>
          </div>

          <div class="metric-cell">
            <span class="metric-label">RTT / Jitter</span>
            <span class="metric-value">
              {telemetry.networkRttMs} / {telemetry.jitterMs} ms
            </span>
          </div>

          <div class="metric-cell">
            <span class="metric-label">Bitrate (Cap)</span>
            <span class="metric-value">
              {telemetry.bitrateMbps} / {selectedBitrateCap} Mbps
            </span>
          </div>

          <div class="metric-cell">
            <span class="metric-label">Loss / FEC Rec</span>
            <span class="metric-value">
              {telemetry.packetLossPct}% / {telemetry.fecRecoveryPct}%
            </span>
          </div>

          {#if hudMode === 'expanded'}
            <div class="metric-cell">
              <span class="metric-label">Encode Lag</span>
              <span class="metric-value">{telemetry.encodeLatencyMs} ms</span>
            </div>

            <div class="metric-cell">
              <span class="metric-label">Delay Gradient Δd</span>
              <span class="metric-value">{telemetry.delayGradientMs} ms</span>
            </div>

            <div class="metric-cell">
              <span class="metric-label">NACK / PLI</span>
              <span class="metric-value">{telemetry.nackCount} / {telemetry.pliCount}</span>
            </div>

            <div class="metric-cell">
              <span class="metric-label">Resolution</span>
              <span class="metric-value">{telemetry.resolution}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Bottom Control Bar -->
    <div class="stream-toolbar">
      <div class="toolbar-left">
        <button
          class="toolbar-btn"
          onclick={() => isHudVisible = !isHudVisible}
          aria-pressed={isHudVisible}
        >
          📊 {isHudVisible ? 'Hide HUD' : 'Show HUD'}
        </button>

        <button
          class="toolbar-btn warn"
          onclick={handleRequestKeyframe}
          title="Force immediate IDR keyframe packet (PLI)"
        >
          🔄 Request Keyframe (PLI)
        </button>

        <button
          class="toolbar-btn danger"
          onclick={handleEmergencyRelease}
          title="Release all held keys and mouse buttons on host OS"
        >
          🛡️ Release Stuck Keys
        </button>

        <button
          class="toolbar-btn {isViewOnly ? 'active-view' : ''}"
          onclick={() => isViewOnly = !isViewOnly}
          title="Toggle between full control and view-only observer mode"
        >
          {isViewOnly ? '🔒 View-Only' : '🎮 Interactive'}
        </button>
      </div>

      <div class="toolbar-right">
        <div class="touch-mode-group">
          <button
            class="mode-btn {touchMode === 'direct' ? 'active' : ''}"
            onclick={() => touchMode = 'direct'}
            title="Direct 1:1 touch coordinate mapping"
          >
            Direct Touch
          </button>
          <button
            class="mode-btn {touchMode === 'trackpad' ? 'active' : ''}"
            onclick={() => touchMode = 'trackpad'}
            title="Relative trackpad mouse navigation"
          >
            Trackpad
          </button>
        </div>

        <button
          class="toolbar-btn {showVirtualGamepad ? 'active-gamepad' : ''}"
          onclick={() => showVirtualGamepad = !showVirtualGamepad}
          title="Toggle On-Screen Virtual Gamepad Overlay"
        >
          🕹️ Gamepad
        </button>

        <div class="slider-group">
          <span class="slider-label">Cap: {selectedBitrateCap} Mbps</span>
          <input
            type="range"
            min="2"
            max="50"
            step="1"
            value={selectedBitrateCap}
            oninput={handleBitrateSlider}
            class="bitrate-slider"
          />
        </div>

        <button
          class="toolbar-btn"
          onclick={toggleFullscreen}
          title="Toggle Fullscreen"
        >
          {isFullscreen ? '🗗 Exit' : '⛶ Fullscreen'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .stream-player-container {
    display: flex;
    flex-direction: column;
    background: #0b0f19;
    border: 1px solid #1f2937;
    border-radius: 12px;
    overflow: hidden;
    position: relative;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .video-viewport {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #030712;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .video-mock-surface {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 20px;
    background: radial-gradient(circle at center, #111827 0%, #030712 100%);
  }

  .stream-center-badge {
    display: flex;
    gap: 8px;
    align-self: flex-start;
  }

  .host-pill, .codec-pill, .res-pill, .view-only-pill, .full-control-pill {
    font-size: 11px;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 9999px;
    background: rgba(31, 41, 55, 0.8);
    backdrop-filter: blur(8px);
    color: #e5e7eb;
    border: 1px solid rgba(75, 85, 99, 0.5);
  }

  .codec-pill {
    background: rgba(14, 165, 233, 0.2);
    color: #38bdf8;
    border-color: rgba(14, 165, 233, 0.4);
  }

  .view-only-pill {
    background: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
    border-color: rgba(245, 158, 11, 0.5);
  }

  .full-control-pill {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
    border-color: rgba(16, 185, 129, 0.5);
  }

  .synthetic-barcode-badge {
    align-self: flex-end;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    background: rgba(0, 0, 0, 0.7);
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid #374151;
  }

  .barcode-matrix {
    font-family: monospace;
    font-size: 14px;
    letter-spacing: 2px;
    color: #10b981;
  }

  .timestamp-label {
    font-family: monospace;
    font-size: 10px;
    color: #9ca3af;
  }

  /* Virtual Gamepad Overlay */
  .virtual-gamepad-overlay {
    position: absolute;
    inset: 40px 30px 60px 30px;
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    pointer-events: none;
  }

  .gamepad-cluster {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: auto;
  }

  .stick-label {
    font-size: 10px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
  }

  .dpad-grid {
    display: grid;
    grid-template-columns: repeat(3, 40px);
    grid-template-rows: repeat(3, 40px);
    gap: 4px;
    background: rgba(15, 23, 42, 0.6);
    border-radius: 50%;
    padding: 8px;
    backdrop-filter: blur(8px);
  }

  .gamepad-btn {
    background: rgba(51, 65, 85, 0.75);
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 8px;
    color: #ffffff;
    font-weight: bold;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    user-select: none;
    transition: background 0.1s, transform 0.1s;
  }

  .gamepad-btn:active {
    background: rgba(14, 165, 233, 0.8);
    transform: scale(0.92);
  }

  .dpad-grid .up { grid-column: 2; grid-row: 1; }
  .dpad-grid .left { grid-column: 1; grid-row: 2; }
  .dpad-grid .center { grid-column: 2; grid-row: 2; background: transparent; border: none; }
  .dpad-grid .right { grid-column: 3; grid-row: 2; }
  .dpad-grid .down { grid-column: 2; grid-row: 3; }

  .face-buttons-grid {
    display: grid;
    grid-template-columns: repeat(3, 40px);
    grid-template-rows: repeat(3, 40px);
    gap: 4px;
    background: rgba(15, 23, 42, 0.6);
    border-radius: 50%;
    padding: 8px;
    backdrop-filter: blur(8px);
  }

  .face-buttons-grid .btn-y { grid-column: 2; grid-row: 1; border-color: #f59e0b; color: #fbbf24; }
  .face-buttons-grid .btn-x { grid-column: 1; grid-row: 2; border-color: #38bdf8; color: #38bdf8; }
  .face-buttons-grid .btn-b { grid-column: 3; grid-row: 2; border-color: #ef4444; color: #f87171; }
  .face-buttons-grid .btn-a { grid-column: 2; grid-row: 3; border-color: #10b981; color: #34d399; }

  .release-toast {
    position: absolute;
    top: 60px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(16, 185, 129, 0.95);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
    padding: 8px 16px;
    border-radius: 9999px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    z-index: 30;
  }

  /* Telemetry HUD Overlay */
  .telemetry-hud {
    position: absolute;
    top: 16px;
    right: 16px;
    background: rgba(15, 23, 42, 0.88);
    backdrop-filter: blur(12px);
    border: 1px solid rgba(51, 65, 85, 0.8);
    border-radius: 8px;
    padding: 12px;
    width: 320px;
    z-index: 20;
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.6);
  }

  .telemetry-hud.compact {
    width: 240px;
  }

  .hud-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(51, 65, 85, 0.5);
    padding-bottom: 8px;
    margin-bottom: 10px;
  }

  .hud-title-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hud-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .hud-title {
    font-size: 12px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .hud-state {
    font-size: 10px;
    font-family: monospace;
    font-weight: 700;
  }

  .hud-controls-group {
    display: flex;
    gap: 4px;
  }

  .hud-btn-mini {
    background: rgba(51, 65, 85, 0.6);
    border: none;
    color: #cbd5e1;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .hud-btn-mini:hover {
    background: rgba(71, 85, 105, 0.8);
    color: #ffffff;
  }

  .hud-metrics-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .metric-cell {
    background: rgba(30, 41, 59, 0.5);
    padding: 6px 8px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
  }

  .metric-label {
    font-size: 9px;
    text-transform: uppercase;
    color: #94a3b8;
    margin-bottom: 2px;
    letter-spacing: 0.5px;
  }

  .metric-value {
    font-size: 13px;
    font-weight: 700;
    font-family: monospace;
    color: #f8fafc;
  }

  .metric-value.optimal {
    color: #10b981;
  }

  .metric-value.warning {
    color: #f59e0b;
  }

  .metric-value.critical {
    color: #ef4444;
  }

  /* Bottom Toolbar */
  .stream-toolbar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 48px;
    background: rgba(15, 23, 42, 0.92);
    backdrop-filter: blur(8px);
    border-top: 1px solid rgba(51, 65, 85, 0.6);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    z-index: 10;
  }

  .toolbar-left, .toolbar-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toolbar-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #e2e8f0;
    font-size: 11px;
    font-weight: 500;
    padding: 5px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .toolbar-btn:hover {
    background: #334155;
    color: #ffffff;
  }

  .toolbar-btn.warn {
    background: rgba(245, 158, 11, 0.15);
    border-color: rgba(245, 158, 11, 0.4);
    color: #fde68a;
  }

  .toolbar-btn.danger {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.4);
    color: #fca5a5;
  }

  .toolbar-btn.danger:hover {
    background: rgba(239, 68, 68, 0.3);
    color: #ffffff;
  }

  .toolbar-btn.active-view {
    background: rgba(245, 158, 11, 0.3);
    border-color: #f59e0b;
    color: #fbbf24;
  }

  .toolbar-btn.active-gamepad {
    background: rgba(14, 165, 233, 0.3);
    border-color: #0284c7;
    color: #38bdf8;
  }

  .touch-mode-group {
    display: flex;
    background: #1e293b;
    border-radius: 6px;
    padding: 2px;
    border: 1px solid #334155;
  }

  .mode-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  .mode-btn.active {
    background: #0ea5e9;
    color: #ffffff;
  }

  .slider-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .slider-label {
    font-size: 11px;
    font-family: monospace;
    color: #cbd5e1;
  }

  .bitrate-slider {
    width: 90px;
    accent-color: #38bdf8;
    cursor: pointer;
  }
</style>
