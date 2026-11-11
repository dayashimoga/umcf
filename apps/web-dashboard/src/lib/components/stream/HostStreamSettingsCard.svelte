<script lang="ts">
  /**
   * Svelte 5 Host Stream Capture & Low-Latency Video Encoder Settings (UMCF S6.P3.SP1)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  interface CaptureSource {
    id: string;
    name: string;
    type: 'monitor' | 'window' | 'virtual';
    resolution: string;
    refreshRateHz: number;
    previewGradient: string;
  }

  interface Props {
    nodeId?: string;
    onSettingsChanged?: (settings: Record<string, unknown>) => void;
  }

  let {
    nodeId = 'local-host-node',
    onSettingsChanged
  }: Props = $props();

  let sources = $state<CaptureSource[]>([
    {
      id: 'monitor-0',
      name: 'Primary Display (LG UltraGear 4K)',
      type: 'monitor',
      resolution: '3840 × 2160',
      refreshRateHz: 120,
      previewGradient: 'linear-gradient(135deg, #1e3a8a, #065f46)'
    },
    {
      id: 'monitor-1',
      name: 'Secondary Display (Dell QHD)',
      type: 'monitor',
      resolution: '2560 × 1440',
      refreshRateHz: 60,
      previewGradient: 'linear-gradient(135deg, #374151, #1f2937)'
    },
    {
      id: 'window-cyberpunk',
      name: 'Cyberpunk 2077 (D3D12 Dedicated)',
      type: 'window',
      resolution: '2560 × 1440',
      refreshRateHz: 120,
      previewGradient: 'linear-gradient(135deg, #991b1b, #78350f)'
    },
    {
      id: 'virtual-headless',
      name: 'Headless Low-Latency Virtual Display',
      type: 'virtual',
      resolution: '1920 × 1080',
      refreshRateHz: 60,
      previewGradient: 'linear-gradient(135deg, #4c1d95, #1e1b4b)'
    }
  ]);

  let selectedSourceId = $state('monitor-0');
  let selectedCodec = $state<'Auto' | 'AV1' | 'HEVC (10-bit HDR)' | 'H.264'>('Auto');
  let targetFps = $state<30 | 60 | 90 | 120>(60);
  let bitrateMbps = $state(15);
  let hideNotifications = $state(true);
  let blankHostScreen = $state(false);
  let isCapturingActive = $state(false);

  // Live Performance Telemetry
  let liveCaptureFps = $state(59.9);
  let liveEncodeLatencyMs = $state(3.2);

  let activeSource = $derived(
    sources.find(s => s.id === selectedSourceId) ?? sources[0]
  );

  let effectiveCodec = $derived(
    selectedCodec === 'Auto' ? 'AV1 (Hardware Accelerated)' : selectedCodec
  );

  function toggleCapture() {
    isCapturingActive = !isCapturingActive;
    if (onSettingsChanged) {
      onSettingsChanged({
        sourceId: selectedSourceId,
        codec: selectedCodec,
        fps: targetFps,
        bitrateMbps,
        hideNotifications,
        blankHostScreen,
        active: isCapturingActive
      });
    }
  }
</script>

<div class="host-stream-card">
  <!-- Header -->
  <header class="card-header">
    <div class="title-group">
      <div class="badge-row">
        <span class="badge badge-node">{nodeId}</span>
        <span class="badge badge-tech">DXGI / PIPEWIRE ZERO-COPY</span>
        <span class="badge badge-status" class:active={isCapturingActive}>
          {isCapturingActive ? 'HOST CAPTURE ACTIVE' : 'STANDBY'}
        </span>
      </div>
      <h2>Low-Latency Display & Game Streaming Host</h2>
    </div>

    <button
      type="button"
      class="btn-toggle"
      class:btn-active={isCapturingActive}
      onclick={toggleCapture}
      aria-label={isCapturingActive ? 'Stop Stream Host' : 'Start Stream Host'}
    >
      {isCapturingActive ? '⏹ Stop Host Streaming' : '▶ Start Host Streaming'}
    </button>
  </header>

  <!-- Real-time Latency & FPS Bar (when active) -->
  {#if isCapturingActive}
    <div class="telemetry-bar">
      <div class="telemetry-item">
        <span class="label">Capture & Encode Latency:</span>
        <span class="value latency-green">{liveEncodeLatencyMs.toFixed(1)} ms (p50)</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Sustained Capture FPS:</span>
        <span class="value">{liveCaptureFps.toFixed(1)} / {targetFps} FPS</span>
      </div>
      <div class="telemetry-item">
        <span class="label">Zero B-Frames / GDR:</span>
        <span class="value text-cyan">Active (Single-Frame VBV)</span>
      </div>
    </div>
  {/if}

  <!-- Capture Source Selection Grid -->
  <section class="sources-section">
    <h3>Select Capture Surface or Window</h3>
    <div class="sources-grid">
      {#each sources as src (src.id)}
        <button
          type="button"
          class="source-card"
          class:selected={selectedSourceId === src.id}
          onclick={() => selectedSourceId = src.id}
          aria-label="Select {src.name}"
        >
          <div class="preview-box" style="background: {src.previewGradient}">
            <div class="preview-badge">{src.type.toUpperCase()}</div>
            <div class="preview-res">{src.resolution} @ {src.refreshRateHz}Hz</div>
          </div>
          <span class="source-name">{src.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- Encoder & Streaming Parameters -->
  <section class="controls-grid">
    <!-- Codec Selection -->
    <div class="control-box">
      <label for="codec-select">Video Codec Engine</label>
      <select id="codec-select" bind:value={selectedCodec}>
        <option value="Auto">Auto (Best Hardware Accelerated)</option>
        <option value="AV1">AV1 (Sub-5ms Ultra-Low Delay)</option>
        <option value="HEVC (10-bit HDR)">HEVC (10-bit HDR BT.2020)</option>
        <option value="H.264">H.264 (Maximum Compatibility)</option>
      </select>
      <small class="helper-text">Resolved backend: {effectiveCodec}</small>
    </div>

    <!-- Target Frame Rate -->
    <div class="control-box">
      <label for="fps-select">Target Stream FPS</label>
      <div class="fps-btn-group" id="fps-select">
        {#each [30, 60, 90, 120] as fpsVal}
          <button
            type="button"
            class="fps-btn"
            class:selected={targetFps === fpsVal}
            onclick={() => targetFps = fpsVal as 30 | 60 | 90 | 120}
          >
            {fpsVal} FPS
          </button>
        {/each}
      </div>
    </div>

    <!-- Target Bitrate Slider -->
    <div class="control-box">
      <div class="slider-label">
        <label for="bitrate-slider">Target Bitrate</label>
        <span class="slider-value">{bitrateMbps} Mbps</span>
      </div>
      <input
        id="bitrate-slider"
        type="range"
        min="2"
        max="50"
        step="1"
        bind:value={bitrateMbps}
      />
      <small class="helper-text">Single-frame VBV buffer: {Math.round((bitrateMbps * 1000 / targetFps) * 1.2)} kb</small>
    </div>

    <!-- Privacy & Security Toggles -->
    <div class="control-box privacy-box">
      <label class="privacy-title">Privacy & Security</label>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={hideNotifications} />
        <span>Hide Sensitive OS Notifications & Overlays</span>
      </label>
      <label class="checkbox-row">
        <input type="checkbox" bind:checked={blankHostScreen} />
        <span>Blank Physical Host Screen During Active Stream</span>
      </label>
    </div>
  </section>
</div>

<style>
  .host-stream-card {
    background-color: #111827;
    border: 1px solid #374151;
    border-radius: 12px;
    padding: 1.5rem;
    color: #f3f4f6;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .badge-row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
  }

  .badge {
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .badge-node {
    background-color: #1f2937;
    color: #9ca3af;
  }

  .badge-tech {
    background-color: #1e3a8a;
    color: #93c5fd;
  }

  .badge-status {
    background-color: #374151;
    color: #d1d5db;
  }

  .badge-status.active {
    background-color: #065f46;
    color: #6ee7b7;
  }

  h2 {
    margin: 0;
    font-size: 1.4rem;
    font-weight: 700;
  }

  h3 {
    margin: 0 0 0.75rem 0;
    font-size: 1rem;
    color: #d1d5db;
  }

  .btn-toggle {
    background-color: #2563eb;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.6rem 1.2rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .btn-toggle:hover {
    background-color: #1d4ed8;
  }

  .btn-toggle.btn-active {
    background-color: #dc2626;
  }

  .telemetry-bar {
    display: flex;
    gap: 2rem;
    background-color: #064e3b;
    border: 1px solid #059669;
    padding: 0.75rem 1rem;
    border-radius: 6px;
  }

  .telemetry-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .telemetry-item .label {
    color: #a7f3d0;
  }

  .telemetry-item .value {
    font-weight: 700;
    color: #ffffff;
  }

  .latency-green {
    color: #34d399 !important;
  }

  .text-cyan {
    color: #67e8f9 !important;
  }

  .sources-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .source-card {
    background: #1f2937;
    border: 2px solid transparent;
    border-radius: 8px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s ease;
  }

  .source-card.selected {
    border-color: #3b82f6;
    background-color: #1e293b;
  }

  .preview-box {
    height: 100px;
    border-radius: 6px;
    position: relative;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .preview-badge {
    background-color: rgba(0, 0, 0, 0.6);
    color: #ffffff;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    align-self: flex-start;
  }

  .preview-res {
    background-color: rgba(0, 0, 0, 0.6);
    color: #d1d5db;
    font-size: 0.7rem;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    align-self: flex-end;
  }

  .source-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #f3f4f6;
  }

  .controls-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1.25rem;
  }

  .control-box {
    background-color: #1f2937;
    padding: 1rem;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .control-box label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #d1d5db;
  }

  select {
    background-color: #111827;
    border: 1px solid #4b5563;
    color: #f3f4f6;
    padding: 0.5rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .helper-text {
    font-size: 0.75rem;
    color: #9ca3af;
  }

  .fps-btn-group {
    display: flex;
    gap: 0.5rem;
  }

  .fps-btn {
    flex: 1;
    background-color: #111827;
    border: 1px solid #4b5563;
    color: #9ca3af;
    padding: 0.4rem;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .fps-btn.selected {
    background-color: #2563eb;
    border-color: #3b82f6;
    color: white;
  }

  .slider-label {
    display: flex;
    justify-content: space-between;
  }

  .slider-value {
    font-weight: 700;
    color: #60a5fa;
  }

  input[type="range"] {
    accent-color: #3b82f6;
  }

  .privacy-title {
    margin-bottom: 0.25rem;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem !important;
    font-weight: normal !important;
    color: #9ca3af !important;
    cursor: pointer;
  }

  .checkbox-row input {
    accent-color: #3b82f6;
  }
</style>

