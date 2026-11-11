<script lang="ts">
  /**
   * Svelte 5 Hardware Media Engines & Transcoding Capabilities Card (UMCF S6.P2.SP1)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  interface CodecCapability {
    codec: string;
    backend: string;
    encoderName: string;
    direction: 'Encode' | 'Decode';
    maxResolution: string;
    supportsBframes: boolean;
    supports10bitHdr: boolean;
    maxConcurrentSessions: number;
    benchmarkFps1080p: number;
    verifiedLive: boolean;
  }

  interface Props {
    nodeId?: string;
    capabilities?: CodecCapability[];
  }

  let {
    nodeId = 'local-node',
    capabilities = [
      {
        codec: 'H.264 / AVC',
        backend: 'Intel QuickSync (QSV)',
        encoderName: 'h264_qsv',
        direction: 'Encode',
        maxResolution: '4096 × 2160',
        supportsBframes: true,
        supports10bitHdr: false,
        maxConcurrentSessions: 4,
        benchmarkFps1080p: 195.0,
        verifiedLive: true
      },
      {
        codec: 'H.265 / HEVC',
        backend: 'Intel QuickSync (QSV)',
        encoderName: 'hevc_qsv',
        direction: 'Encode',
        maxResolution: '7680 × 4320',
        supportsBframes: true,
        supports10bitHdr: true,
        maxConcurrentSessions: 4,
        benchmarkFps1080p: 160.0,
        verifiedLive: true
      },
      {
        codec: 'AV1',
        backend: 'Intel QuickSync (QSV)',
        encoderName: 'av1_qsv',
        direction: 'Encode',
        maxResolution: '7680 × 4320',
        supportsBframes: false,
        supports10bitHdr: true,
        maxConcurrentSessions: 4,
        benchmarkFps1080p: 140.0,
        verifiedLive: true
      },
      {
        codec: 'Opus Audio',
        backend: 'Software CPU (libopus)',
        encoderName: 'libopus',
        direction: 'Encode',
        maxResolution: '48 kHz Stereo / 7.1',
        supportsBframes: false,
        supports10bitHdr: false,
        maxConcurrentSessions: 32,
        benchmarkFps1080p: 1200.0,
        verifiedLive: true
      }
    ]
  }: Props = $props();

  let filterBackend = $state<'ALL' | 'HARDWARE' | 'SOFTWARE'>('ALL');

  let filteredCodecs = $derived(
    capabilities.filter((c) => {
      if (filterBackend === 'HARDWARE') return !c.backend.includes('Software');
      if (filterBackend === 'SOFTWARE') return c.backend.includes('Software');
      return true;
    })
  );

  let hwEncoderCount = $derived(
    capabilities.filter((c) => !c.backend.includes('Software')).length
  );
</script>

<div class="media-engines-card" role="region" aria-label="Hardware Media Engines">
  <header class="card-header">
    <div class="header-titles">
      <h3>Hardware Media Engines & Codecs</h3>
      <span class="badge hw-badge">{hwEncoderCount} HW Accelerated</span>
      <span class="node-id">Node: {nodeId}</span>
    </div>
    <div class="filter-controls">
      <button
        class="filter-btn"
        class:active={filterBackend === 'ALL'}
        onclick={() => filterBackend = 'ALL'}
      >
        All ({capabilities.length})
      </button>
      <button
        class="filter-btn"
        class:active={filterBackend === 'HARDWARE'}
        onclick={() => filterBackend = 'HARDWARE'}
      >
        Hardware Only
      </button>
      <button
        class="filter-btn"
        class:active={filterBackend === 'SOFTWARE'}
        onclick={() => filterBackend = 'SOFTWARE'}
      >
        Software Fallback
      </button>
    </div>
  </header>

  <div class="codec-grid">
    {#each filteredCodecs as cap}
      <div class="codec-item" class:hardware={!cap.backend.includes('Software')}>
        <div class="item-header">
          <span class="codec-title">{cap.codec}</span>
          <span class="direction-pill">{cap.direction}</span>
        </div>

        <div class="backend-info">
          <code>{cap.encoderName}</code>
          <span class="backend-tag">{cap.backend}</span>
        </div>

        <div class="specs-row">
          <span class="res-spec">Max: {cap.maxResolution}</span>
          <span class="fps-spec">{cap.benchmarkFps1080p.toFixed(0)} FPS (1080p)</span>
        </div>

        <div class="features-row">
          {#if cap.supports10bitHdr}
            <span class="feat-badge hdr-badge">10-Bit HDR</span>
          {/if}
          {#if cap.supportsBframes}
            <span class="feat-badge bframe-badge">B-Frames</span>
          {/if}
          <span class="feat-badge session-badge">Max {cap.maxConcurrentSessions} Sessions</span>
          {#if cap.verifiedLive}
            <span class="feat-badge verified-badge">✓ Verified Live</span>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <footer class="card-footer">
    <span class="fallback-notice">
      ⚡ Automatic Fallback Active: Transcoding transparently cascades from saturated HW encoders to deterministic CPU fallbacks (libx264/libx265/libsvtav1).
    </span>
  </footer>
</div>

<style>
  .media-engines-card {
    border: 1px solid var(--border-color, #2d2d38);
    border-radius: 8px;
    background: #181820;
    color: #e5e7eb;
    padding: 1.25rem;
    box-sizing: border-box;
  }
  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid #282834;
    padding-bottom: 0.75rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    gap: 0.75rem;
  }
  .header-titles {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .header-titles h3 {
    margin: 0;
    font-size: 1.15rem;
  }
  .badge {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: 600;
  }
  .hw-badge {
    background: #065f46;
    color: #6ee7b7;
  }
  .node-id {
    font-size: 0.8rem;
    color: #9ca3af;
  }
  .filter-controls {
    display: flex;
    gap: 0.4rem;
  }
  .filter-btn {
    background: #252530;
    border: 1px solid #3c3c4a;
    color: #9ca3af;
    border-radius: 4px;
    padding: 0.35rem 0.65rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .filter-btn.active {
    background: #3b82f6;
    color: white;
    border-color: #2563eb;
  }
  .codec-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .codec-item {
    background: #121218;
    border: 1px solid #282834;
    border-radius: 6px;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .codec-item.hardware {
    border-left: 3px solid #10b981;
  }
  .item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .codec-title {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .direction-pill {
    background: #2a2a36;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
    color: #93c5fd;
  }
  .backend-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
  }
  code {
    background: #09090e;
    padding: 0.15rem 0.35rem;
    border-radius: 3px;
    color: #a78bfa;
    font-size: 0.75rem;
    font-family: monospace;
  }
  .backend-tag {
    color: #9ca3af;
    font-size: 0.75rem;
  }
  .specs-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    color: #d1d5db;
  }
  .fps-spec {
    font-weight: 600;
    color: #34d399;
  }
  .features-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.25rem;
  }
  .feat-badge {
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 500;
  }
  .hdr-badge {
    background: #78350f;
    color: #fde68a;
  }
  .bframe-badge {
    background: #1e3a8a;
    color: #bfdbfe;
  }
  .session-badge {
    background: #374151;
    color: #e5e7eb;
  }
  .verified-badge {
    background: #064e3b;
    color: #6ee7b7;
  }
  .card-footer {
    border-top: 1px solid #282834;
    padding-top: 0.75rem;
    font-size: 0.75rem;
    color: #9ca3af;
  }
  .fallback-notice {
    color: #a5b4fc;
  }
</style>

