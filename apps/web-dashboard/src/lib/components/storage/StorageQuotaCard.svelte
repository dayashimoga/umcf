<script lang="ts">
  /**
   * Svelte 5 Storage Quota Card & Stacked Allocation Bar (UMCF S7.P1.SP1)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  export interface StorageAllocationBreakdown {
    pinnedBytes: number;
    modelShardsBytes: number;
    cachedChunksBytes: number;
    inFlightReservedBytes: number;
    hostFreeBytes: number;
    hostSafetyFloorBytes: number;
    configuredMaxBytes: number;
  }

  interface Props {
    nodeId?: string;
    initialMaxGb?: number;
    onQuotaChanged?: (newMaxBytes: number) => void;
    onPurgeCacheRequested?: () => Promise<number>;
  }

  let {
    nodeId = 'local-storage-node',
    initialMaxGb = 32,
    onQuotaChanged,
    onPurgeCacheRequested,
  }: Props = $props();

  const GB = 1024 * 1024 * 1024;
  const MB = 1024 * 1024;

  let configuredMaxGb = $state<number>(initialMaxGb);
  let isPurging = $state<boolean>(false);
  let purgeMessage = $state<string | null>(null);

  // Live storage breakdown state
  let breakdown = $state<StorageAllocationBreakdown>({
    pinnedBytes: 4.2 * GB,
    modelShardsBytes: 12.8 * GB,
    cachedChunksBytes: 6.4 * GB,
    inFlightReservedBytes: 0.8 * GB,
    hostFreeBytes: 45.0 * GB,
    hostSafetyFloorBytes: 5.0 * GB,
    configuredMaxBytes: initialMaxGb * GB,
  });

  // Derived metrics
  let totalUsedBytes = $derived(
    breakdown.pinnedBytes +
    breakdown.modelShardsBytes +
    breakdown.cachedChunksBytes +
    breakdown.inFlightReservedBytes
  );

  let currentLimitBytes = $derived(configuredMaxGb * GB);

  let usedPct = $derived(
    Math.min(100, Number(((totalUsedBytes / currentLimitBytes) * 100).toFixed(1)))
  );

  let pinnedPct = $derived(Number(((breakdown.pinnedBytes / currentLimitBytes) * 100).toFixed(1)));
  let modelsPct = $derived(Number(((breakdown.modelShardsBytes / currentLimitBytes) * 100).toFixed(1)));
  let cachePct = $derived(Number(((breakdown.cachedChunksBytes / currentLimitBytes) * 100).toFixed(1)));
  let inFlightPct = $derived(Number(((breakdown.inFlightReservedBytes / currentLimitBytes) * 100).toFixed(1)));

  function handleSliderChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    configuredMaxGb = val;
    breakdown.configuredMaxBytes = val * GB;
    if (onQuotaChanged) {
      onQuotaChanged(val * GB);
    }
  }

  async function handlePurgeCache() {
    isPurging = true;
    try {
      let freed = breakdown.cachedChunksBytes;
      if (onPurgeCacheRequested) {
        freed = await onPurgeCacheRequested();
      } else {
        // Local simulation fallback
        await new Promise((r) => setTimeout(r, 400));
        breakdown.cachedChunksBytes = 0;
      }
      purgeMessage = `Purged ${(freed / GB).toFixed(2)} GB unpinned cache`;
    } catch (err) {
      purgeMessage = `Purge error: ${err}`;
    } finally {
      isPurging = false;
      setTimeout(() => {
        purgeMessage = null;
      }, 3000);
    }
  }
</script>

<div class="quota-card" role="region" aria-label="Storage quota management">
  <div class="card-header">
    <div class="title-group">
      <span class="storage-icon">💾</span>
      <div>
        <h3 class="card-title">Content-Addressed Storage Quota</h3>
        <span class="node-subtitle">Node: {nodeId} · FastCDC + BLAKE3</span>
      </div>
    </div>
    <button
      class="purge-btn"
      onclick={handlePurgeCache}
      disabled={isPurging || breakdown.cachedChunksBytes === 0}
      title="Evict all unpinned opportunistic chunk cache"
    >
      {isPurging ? 'Purging…' : '🧹 Purge Cache'}
    </button>
  </div>

  {#if purgeMessage}
    <div class="purge-toast" role="status">
      ✨ {purgeMessage}
    </div>
  {/if}

  <!-- Stacked Allocation Bar -->
  <div class="allocation-section">
    <div class="allocation-labels">
      <span class="used-label">
        <strong>{(totalUsedBytes / GB).toFixed(2)} GB</strong> used of <strong>{configuredMaxGb} GB</strong> ({usedPct}%)
      </span>
      <span class="host-free-label">
        Host Free: {(breakdown.hostFreeBytes / GB).toFixed(1)} GB (Floor: {(breakdown.hostSafetyFloorBytes / GB).toFixed(1)} GB)
      </span>
    </div>

    <div
      class="stacked-bar-track"
      role="progressbar"
      aria-valuenow={usedPct}
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <div class="bar-segment pinned" style="width: {pinnedPct}%" title="Pinned Data: {(breakdown.pinnedBytes / GB).toFixed(2)} GB"></div>
      <div class="bar-segment models" style="width: {modelsPct}%" title="AI Model Shards: {(breakdown.modelShardsBytes / GB).toFixed(2)} GB"></div>
      <div class="bar-segment cache" style="width: {cachePct}%" title="Cached Chunks: {(breakdown.cachedChunksBytes / GB).toFixed(2)} GB"></div>
      <div class="bar-segment inflight" style="width: {inFlightPct}%" title="In-Flight Reserved: {(breakdown.inFlightReservedBytes / MB).toFixed(0)} MB"></div>
    </div>

    <!-- Category Legend -->
    <div class="legend-grid">
      <div class="legend-item">
        <span class="legend-dot pinned"></span>
        <span class="legend-name">Pinned ({(breakdown.pinnedBytes / GB).toFixed(1)} GB)</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot models"></span>
        <span class="legend-name">Models ({(breakdown.modelShardsBytes / GB).toFixed(1)} GB)</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot cache"></span>
        <span class="legend-name">Cache ({(breakdown.cachedChunksBytes / GB).toFixed(1)} GB)</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot inflight"></span>
        <span class="legend-name">In-Flight ({(breakdown.inFlightReservedBytes / MB).toFixed(0)} MB)</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot floor"></span>
        <span class="legend-name">Host Floor (15%)</span>
      </div>
    </div>
  </div>

  <!-- Interactive Quota Ceiling Slider -->
  <div class="slider-section">
    <div class="slider-header">
      <label for="storage-quota-slider" class="slider-title">Configured Storage Cap</label>
      <span class="slider-val">{configuredMaxGb} GB</span>
    </div>
    <input
      id="storage-quota-slider"
      type="range"
      min="1"
      max="128"
      step="1"
      value={configuredMaxGb}
      oninput={handleSliderChange}
      class="quota-slider"
    />
    <div class="slider-ticks">
      <span>1 GB</span>
      <span>16 GB</span>
      <span>32 GB</span>
      <span>64 GB</span>
      <span>128 GB</span>
    </div>
  </div>
</div>

<style>
  .quota-card {
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 12px;
    padding: 20px;
    color: #f8fafc;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    max-width: 680px;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .storage-icon {
    font-size: 24px;
  }

  .card-title {
    font-size: 15px;
    font-weight: 700;
    margin: 0;
    color: #f1f5f9;
  }

  .node-subtitle {
    font-size: 11px;
    color: #94a3b8;
  }

  .purge-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    font-size: 11px;
    font-weight: 600;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .purge-btn:hover:not(:disabled) {
    background: #334155;
    color: #ffffff;
  }

  .purge-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .purge-toast {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.4);
    color: #6ee7b7;
    font-size: 12px;
    padding: 8px 12px;
    border-radius: 6px;
    margin-bottom: 14px;
  }

  .allocation-section {
    background: #0b0f19;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 14px;
    margin-bottom: 20px;
  }

  .allocation-labels {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    margin-bottom: 8px;
  }

  .used-label {
    color: #e2e8f0;
  }

  .host-free-label {
    color: #94a3b8;
    font-family: monospace;
    font-size: 11px;
  }

  .stacked-bar-track {
    display: flex;
    height: 12px;
    background: #1e293b;
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 12px;
  }

  .bar-segment {
    height: 100%;
    transition: width 0.3s ease;
  }

  .bar-segment.pinned, .legend-dot.pinned { background: #10b981; }
  .bar-segment.models, .legend-dot.models { background: #3b82f6; }
  .bar-segment.cache, .legend-dot.cache { background: #6366f1; }
  .bar-segment.inflight, .legend-dot.inflight { background: #f59e0b; }
  .legend-dot.floor { background: #ef4444; }

  .legend-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 8px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: #cbd5e1;
  }

  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .slider-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .slider-title {
    font-size: 12px;
    font-weight: 600;
    color: #cbd5e1;
  }

  .slider-val {
    font-family: monospace;
    font-size: 13px;
    font-weight: 700;
    color: #38bdf8;
  }

  .quota-slider {
    width: 100%;
    accent-color: #38bdf8;
    cursor: pointer;
  }

  .slider-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #64748b;
    font-family: monospace;
  }
</style>

