<script lang="ts">
  /**
   * Svelte 5 Ultra-Low-Latency WebRTC/QUIC Stream Player & Telemetry HUD (UMCF S6.P3.SP2)
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
    onKeyframeRequested?: (streamId: number) => void;
    onBitrateLimitChanged?: (limitMbps: number) => void;
  }

  let {
    streamId = 1,
    hostName = 'Gaming-Host-Desktop',
    codec = 'H264',
    initialBitrateMbps = 25,
    onKeyframeRequested,
    onBitrateLimitChanged,
  }: Props = $props();

  // Reactive state using Svelte 5 runes
  let isConnected = $state<boolean>(true);
  let isHudVisible = $state<boolean>(true);
  let hudMode = $state<'compact' | 'expanded'>('expanded');
  let selectedBitrateCap = $state<number>(initialBitrateMbps);
  let autoAbrEnabled = $state<boolean>(true);
  let isFullscreen = $state<boolean>(false);
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

  // Simulation timer for realistic telemetry jitter
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
  aria-label="Low-latency stream display"
>
  <!-- Mock video stream viewport (Canvas / WebGL / WebCodecs surface) -->
  <div class="video-viewport">
    <div class="video-mock-surface">
      <div class="stream-center-badge">
        <span class="host-pill">🖥️ {hostName}</span>
        <span class="codec-pill">{codec} 60 FPS</span>
        <span class="res-pill">{telemetry.resolution}</span>
      </div>

      <!-- Watermark Barcode & Timestamp representation -->
      <div class="synthetic-barcode-badge">
        <span class="barcode-matrix">█ ▄ █ ▌ █ █ ▌ ▄</span>
        <span class="timestamp-label">PTS: {(telemetry.frameSeq * 16666).toLocaleString()} µs</span>
      </div>
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

        <label class="auto-abr-checkbox">
          <input
            type="checkbox"
            bind:checked={autoAbrEnabled}
          />
          Auto-ABR
        </label>
      </div>

      <div class="toolbar-right">
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
          {isFullscreen ? '🗗 Exit Fullscreen' : '⛶ Fullscreen'}
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

  .host-pill, .codec-pill, .res-pill {
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
    gap: 12px;
  }

  .toolbar-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #e2e8f0;
    font-size: 12px;
    font-weight: 500;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .toolbar-btn:hover {
    background: #334155;
    color: #ffffff;
  }

  .toolbar-btn.warn {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.4);
    color: #fca5a5;
  }

  .toolbar-btn.warn:hover {
    background: rgba(239, 68, 68, 0.3);
    color: #ffffff;
  }

  .auto-abr-checkbox {
    font-size: 12px;
    color: #94a3b8;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
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
    width: 100px;
    accent-color: #38bdf8;
    cursor: pointer;
  }
</style>
