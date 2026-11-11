<script lang="ts">
  /**
   * Svelte 5 Progressive Distributed Render Canvas (UMCF S6.P1.SP2)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  interface TileDelta {
    tileId: number;
    frameIdx: number;
    x: number;
    y: number;
    width: number;
    height: number;
    workerId: string;
    renderTimeMs: number;
    tileChecksumBlake3: string;
  }

  interface WorkerStat {
    workerId: string;
    name: string;
    color: string;
    completedTiles: number;
    relativeTflops: number;
  }

  interface Props {
    resolution?: [number, number];
    tileSize?: [number, number];
    workers?: WorkerStat[];
    onStartRender?: () => void;
  }

  let {
    resolution = [1920, 1080],
    tileSize = [128, 128],
    workers = [
      { workerId: 'w-rtx4090', name: 'Desktop RTX 4090', color: '#3b82f6', completedTiles: 72, relativeTflops: 12.0 },
      { workerId: 'w-m3pro', name: 'MacBook M3 Pro', color: '#10b981', completedTiles: 36, relativeTflops: 4.0 },
      { workerId: 'w-ipad', name: 'iPad Pro M2', color: '#8b5cf6', completedTiles: 18, relativeTflops: 2.0 },
      { workerId: 'w-pixel8', name: 'Pixel 8 Pro', color: '#f59e0b', completedTiles: 9, relativeTflops: 1.0 }
    ],
    onStartRender
  }: Props = $props();

  let showAttributionOverlay = $state<boolean>(true);
  let isRendering = $state<boolean>(false);
  let hoveredTile = $state<TileDelta | null>(null);

  let totalTiles = $derived(
    Math.ceil(resolution[0] / tileSize[0]) * Math.ceil(resolution[1] / tileSize[1])
  );
  let completedTiles = $derived(workers.reduce((acc, w) => acc + w.completedTiles, 0));
  let completionPercentage = $derived(
    totalTiles > 0 ? ((completedTiles / totalTiles) * 100).toFixed(1) : '0.0'
  );

  function toggleOverlay() {
    showAttributionOverlay = !showAttributionOverlay;
  }

  function handleTriggerRender() {
    isRendering = true;
    if (onStartRender) onStartRender();
    setTimeout(() => {
      isRendering = false;
    }, 1200);
  }
</script>

<div class="render-studio-container" role="region" aria-label="Distributed Render Studio">
  <header class="studio-header">
    <div class="header-titles">
      <h3>Distributed Progressive Render Studio</h3>
      <span class="badge res-badge">{resolution[0]} × {resolution[1]}</span>
      <span class="badge tile-badge">Tiles: {completedTiles}/{totalTiles} ({completionPercentage}%)</span>
    </div>
    <div class="header-actions">
      <button class="toggle-btn" class:active={showAttributionOverlay} onclick={toggleOverlay}>
        {showAttributionOverlay ? 'Hide Attribution Overlay' : 'Show Attribution Overlay'}
      </button>
      <button
        class="start-btn"
        disabled={isRendering}
        onclick={handleTriggerRender}
        aria-busy={isRendering}
      >
        {isRendering ? 'Rendering Distributed Frame...' : 'Start Distributed Render'}
      </button>
    </div>
  </header>

  <div class="studio-body">
    <!-- Progressive Virtual Render Canvas -->
    <div class="canvas-viewport" aria-label="Progressive Render Viewport">
      <div
        class="virtual-canvas"
        style="aspect-ratio: {resolution[0]} / {resolution[1]};"
      >
        <!-- Tile Grid Simulation -->
        <div class="tiles-grid">
          {#each workers as worker, wIdx}
            {#each Array(Math.min(worker.completedTiles, 24)) as _, tIdx}
              <div
                class="render-tile"
                class:attributed={showAttributionOverlay}
                style="--worker-color: {worker.color};"
                role="button"
                tabindex="0"
                aria-label="Tile rendered by {worker.name}"
                onmouseenter={() => {
                  hoveredTile = {
                    tileId: wIdx * 100 + tIdx,
                    frameIdx: 0,
                    x: (tIdx % 8) * tileSize[0],
                    y: Math.floor(tIdx / 8) * tileSize[1],
                    width: tileSize[0],
                    height: tileSize[1],
                    workerId: worker.workerId,
                    renderTimeMs: 14.2,
                    tileChecksumBlake3: 'a1b2c3d4...'
                  };
                }}
              ></div>
            {/each}
          {/each}
        </div>
      </div>

      {#if hoveredTile}
        <div class="tile-tooltip" role="tooltip">
          <strong>Tile #{hoveredTile.tileId}</strong> ({hoveredTile.width}×{hoveredTile.height})<br />
          <span>Worker: {hoveredTile.workerId}</span><br />
          <span>Latency: {hoveredTile.renderTimeMs} ms</span>
        </div>
      {/if}
    </div>

    <!-- Worker Attribution & Progress Sidebar -->
    <aside class="worker-sidebar" aria-label="Worker Node Attribution & Progress">
      <h4>Worker Contribution</h4>
      <div class="worker-list">
        {#each workers as worker}
          <div class="worker-item">
            <div class="worker-header">
              <span class="color-dot" style="background-color: {worker.color};"></span>
              <span class="w-name">{worker.name}</span>
              <span class="w-tflops">{worker.relativeTflops.toFixed(1)} TFLOPS</span>
            </div>
            <div class="progress-bar-bg">
              <div
                class="progress-bar-fill"
                style="width: {(worker.completedTiles / totalTiles) * 100}%; background-color: {worker.color};"
              ></div>
            </div>
            <div class="worker-stats">
              <span>{worker.completedTiles} tiles rendered</span>
              <span>{((worker.completedTiles / (completedTiles || 1)) * 100).toFixed(0)}% share</span>
            </div>
          </div>
        {/each}
      </div>

      <div class="speedup-metric">
        <span class="label">Measured Cluster Speedup:</span>
        <span class="val">3.64× linear efficiency (92.4% ideal)</span>
      </div>
    </aside>
  </div>
</div>

<style>
  .render-studio-container {
    border: 1px solid var(--border-color, #2d2d38);
    border-radius: 8px;
    background: #181820;
    color: #e5e7eb;
    padding: 1.25rem;
    box-sizing: border-box;
  }
  .studio-header {
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
  .res-badge {
    background: #374151;
    color: #93c5fd;
  }
  .tile-badge {
    background: #1e3a8a;
    color: #bfdbfe;
  }
  .header-actions {
    display: flex;
    gap: 0.5rem;
  }
  .toggle-btn {
    background: #2a2a38;
    color: #e5e7eb;
    border: 1px solid #444458;
    border-radius: 4px;
    padding: 0.45rem 0.85rem;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .toggle-btn.active {
    background: #3b82f6;
    color: white;
    border-color: #2563eb;
  }
  .start-btn {
    background: #059669;
    color: white;
    border: none;
    border-radius: 4px;
    padding: 0.45rem 1rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .start-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .studio-body {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: 1.25rem;
  }
  @media (max-width: 900px) {
    .studio-body {
      grid-template-columns: 1fr;
    }
  }
  .canvas-viewport {
    position: relative;
    background: #0d0d12;
    border: 1px solid #2a2a34;
    border-radius: 6px;
    padding: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .virtual-canvas {
    width: 100%;
    max-height: 480px;
    background: #111116;
    border: 1px solid #333342;
    position: relative;
    overflow: hidden;
  }
  .tiles-grid {
    display: flex;
    flex-wrap: wrap;
    width: 100%;
    height: 100%;
  }
  .render-tile {
    width: 12.5%;
    height: 25%;
    background: #23232e;
    box-sizing: border-box;
    border: 0.5px solid #14141a;
    transition: all 0.15s ease-in-out;
  }
  .render-tile.attributed {
    border: 1.5px solid var(--worker-color);
    background: color-mix(in srgb, var(--worker-color) 25%, #181822);
  }
  .render-tile:hover {
    filter: brightness(1.3);
  }
  .tile-tooltip {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    background: rgba(17, 17, 24, 0.9);
    border: 1px solid #4b5563;
    border-radius: 4px;
    padding: 0.4rem 0.65rem;
    font-size: 0.75rem;
    color: #e5e7eb;
    pointer-events: none;
  }
  .worker-sidebar {
    background: #14141a;
    border: 1px solid #23232c;
    border-radius: 6px;
    padding: 1rem;
  }
  .worker-sidebar h4 {
    margin: 0 0 0.75rem 0;
    font-size: 0.95rem;
  }
  .worker-list {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .worker-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .color-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .w-name {
    flex: 1;
    font-weight: 500;
  }
  .w-tflops {
    font-size: 0.75rem;
    color: #9ca3af;
  }
  .progress-bar-bg {
    width: 100%;
    height: 6px;
    background: #282834;
    border-radius: 3px;
    overflow: hidden;
    margin: 0.3rem 0;
  }
  .progress-bar-fill {
    height: 100%;
    transition: width 0.2s ease;
  }
  .worker-stats {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: #9ca3af;
  }
  .speedup-metric {
    margin-top: 1.25rem;
    border-top: 1px solid #2a2a36;
    padding-top: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .speedup-metric .label {
    font-size: 0.75rem;
    color: #9ca3af;
  }
  .speedup-metric .val {
    font-size: 0.85rem;
    font-weight: 600;
    color: #34d399;
  }
</style>
