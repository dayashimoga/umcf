<script lang="ts">
  /**
   * Svelte 5 Segment-Parallel Video Transcode Pipeline & Timeline View (UMCF S6.P2.SP2)
   * Pure Svelte 5 Runes ($state, $derived, $effect)
   */

  interface GopSegment {
    segmentIndex: number;
    startPts: number;
    endPts: number;
    startSec: number;
    endSec: number;
    expectedFrames: number;
    transcodedFrames: number;
    assignedNode: string;
    nodeColor: string;
    encoderBackend: string;
    status: 'pending' | 'transcoding' | 'completed' | 'stitching';
    fps: number;
  }

  interface WorkerNode {
    id: string;
    name: string;
    backend: string;
    fps: number;
    color: string;
    assignedSegments: number;
  }

  interface Props {
    jobId?: string;
    sourceFilename?: string;
    totalDurationSec?: number;
    totalFrames?: number;
    targetCodec?: string;
  }

  let {
    jobId = 'job-media-4k60-001',
    sourceFilename = '4k60_master_prores.mov',
    totalDurationSec = 30.0,
    totalFrames = 1800,
    targetCodec = 'H.264 (High Profile, Level 4.1, Closed GOP)'
  }: Props = $props();

  // Cluster Workers State
  let workers = $state<WorkerNode[]>([
    {
      id: 'node-desktop-rtx4090',
      name: 'Workstation RTX 4090',
      backend: 'NVENC (h264_nvenc)',
      fps: 245.0,
      color: '#10b981', // green
      assignedSegments: 2
    },
    {
      id: 'node-mac-m2pro',
      name: 'MacBook Pro M2 Pro',
      backend: 'VideoToolbox (h264_videotoolbox)',
      fps: 165.0,
      color: '#3b82f6', // blue
      assignedSegments: 2
    },
    {
      id: 'node-laptop-qsv',
      name: 'Intel Core Ultra (QSV)',
      backend: 'QuickSync (h264_qsv)',
      fps: 140.0,
      color: '#8b5cf6', // purple
      assignedSegments: 1
    },
    {
      id: 'node-server-cpu',
      name: 'Threadripper 3970X (CPU)',
      backend: 'Software (libx264)',
      fps: 68.0,
      color: '#f59e0b', // amber
      assignedSegments: 1
    }
  ]);

  // GOP Segments State
  let segments = $state<GopSegment[]>([
    {
      segmentIndex: 0,
      startPts: 0,
      endPts: 450000,
      startSec: 0.0,
      endSec: 5.0,
      expectedFrames: 300,
      transcodedFrames: 300,
      assignedNode: 'node-desktop-rtx4090',
      nodeColor: '#10b981',
      encoderBackend: 'h264_nvenc',
      status: 'completed',
      fps: 242.0
    },
    {
      segmentIndex: 1,
      startPts: 450000,
      endPts: 900000,
      startSec: 5.0,
      endSec: 10.0,
      expectedFrames: 300,
      transcodedFrames: 300,
      assignedNode: 'node-desktop-rtx4090',
      nodeColor: '#10b981',
      encoderBackend: 'h264_nvenc',
      status: 'completed',
      fps: 246.0
    },
    {
      segmentIndex: 2,
      startPts: 900000,
      endPts: 1350000,
      startSec: 10.0,
      endSec: 15.0,
      expectedFrames: 300,
      transcodedFrames: 285,
      assignedNode: 'node-mac-m2pro',
      nodeColor: '#3b82f6',
      encoderBackend: 'h264_videotoolbox',
      status: 'transcoding',
      fps: 164.0
    },
    {
      segmentIndex: 3,
      startPts: 1350000,
      endPts: 1800000,
      startSec: 15.0,
      endSec: 20.0,
      expectedFrames: 300,
      transcodedFrames: 270,
      assignedNode: 'node-mac-m2pro',
      nodeColor: '#3b82f6',
      encoderBackend: 'h264_videotoolbox',
      status: 'transcoding',
      fps: 166.0
    },
    {
      segmentIndex: 4,
      startPts: 1800000,
      endPts: 2250000,
      startSec: 20.0,
      endSec: 25.0,
      expectedFrames: 300,
      transcodedFrames: 250,
      assignedNode: 'node-laptop-qsv',
      nodeColor: '#8b5cf6',
      encoderBackend: 'h264_qsv',
      status: 'transcoding',
      fps: 141.0
    },
    {
      segmentIndex: 5,
      startPts: 2250000,
      endPts: 2700000,
      startSec: 25.0,
      endSec: 30.0,
      expectedFrames: 300,
      transcodedFrames: 195,
      assignedNode: 'node-server-cpu',
      nodeColor: '#f59e0b',
      encoderBackend: 'libx264',
      status: 'transcoding',
      fps: 67.5
    }
  ]);

  let selectedSegment = $state<GopSegment | null>(null);
  let isTranscodingActive = $state(true);

  // Derived Aggregate Metrics
  let totalTranscodedFrames = $derived(
    segments.reduce((acc, s) => acc + s.transcodedFrames, 0)
  );

  let overallProgressPercent = $derived(
    Math.min(100.0, (totalTranscodedFrames / totalFrames) * 100.0)
  );

  let aggregateClusterFps = $derived(
    workers.reduce((acc, w) => acc + w.fps, 0)
  );

  let speedupRealtimeFactor = $derived(
    (aggregateClusterFps / 60.0).toFixed(1)
  );

  let isStitchingReady = $derived(
    totalTranscodedFrames === totalFrames
  );

  function selectSegment(seg: GopSegment) {
    selectedSegment = seg;
  }

  function toggleTranscode() {
    isTranscodingActive = !isTranscodingActive;
  }
</script>

<div class="transcode-pipeline-card">
  <!-- Header & Status Bar -->
  <header class="pipeline-header">
    <div class="title-group">
      <div class="badge-row">
        <span class="badge badge-job">JOB: {jobId}</span>
        <span class="badge badge-codec">{targetCodec}</span>
        <span class="badge badge-status" class:active={isTranscodingActive}>
          {isTranscodingActive ? 'PARALLEL ENCODE ACTIVE' : 'PAUSED'}
        </span>
      </div>
      <h2>{sourceFilename}</h2>
    </div>

    <div class="action-group">
      <button
        type="button"
        class="btn-toggle"
        onclick={toggleTranscode}
        aria-label={isTranscodingActive ? 'Pause Transcode' : 'Resume Transcode'}
      >
        {isTranscodingActive ? '⏸ Pause Cluster' : '▶ Resume Cluster'}
      </button>
    </div>
  </header>

  <!-- Speedup & Cluster Metrics Strip -->
  <section class="metrics-strip">
    <div class="metric-block">
      <span class="label">Aggregate Cluster Speed</span>
      <span class="value speedup-highlight">
        {Math.round(aggregateClusterFps)} FPS
        <small>({speedupRealtimeFactor}x real-time)</small>
      </span>
    </div>

    <div class="metric-block">
      <span class="label">Total Progress</span>
      <span class="value">
        {totalTranscodedFrames} / {totalFrames} frames
        <small>({overallProgressPercent.toFixed(1)}%)</small>
      </span>
    </div>

    <div class="metric-block">
      <span class="label">A/V Sync Verification</span>
      <span class="value text-success">
        &lt; 0.05 ms drift
        <small>(Sample-Accurate)</small>
      </span>
    </div>

    <div class="metric-block">
      <span class="label">Stitch Engine</span>
      <span class="value" class:text-ready={isStitchingReady}>
        {isStitchingReady ? '✓ Ready to Mux' : 'Rebasing PTS / DTS'}
      </span>
    </div>
  </section>

  <!-- Visual Timeline Bar Divided into GOP Segments -->
  <section class="timeline-container">
    <div class="timeline-meta">
      <span>00:00.00</span>
      <span class="timeline-title">GOP Keyframe Boundary Timeline ({segments.length} Segments)</span>
      <span>{totalDurationSec.toFixed(2)}s</span>
    </div>

    <div class="timeline-bar" role="group" aria-label="GOP Video Segments">
      {#each segments as seg (seg.segmentIndex)}
        {@const segWidth = ((seg.endSec - seg.startSec) / totalDurationSec) * 100}
        {@const segProgress = (seg.transcodedFrames / seg.expectedFrames) * 100}
        <button
          type="button"
          class="segment-segment-btn"
          style="width: {segWidth}%; border-bottom-color: {seg.nodeColor};"
          class:selected={selectedSegment?.segmentIndex === seg.segmentIndex}
          onclick={() => selectSegment(seg)}
          aria-label="Segment {seg.segmentIndex}: {seg.startSec}s to {seg.endSec}s on {seg.assignedNode}"
        >
          <div
            class="segment-fill"
            style="width: {segProgress}%; background-color: {seg.nodeColor};"
          ></div>
          <span class="segment-label">S{seg.segmentIndex}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- Segment Detail Drawer (if selected) -->
  {#if selectedSegment}
    <div class="segment-detail-drawer" role="region" aria-label="Segment Details">
      <div class="drawer-header">
        <h4>Segment #{selectedSegment.segmentIndex} Inspection</h4>
        <button type="button" class="btn-close" onclick={() => selectedSegment = null}>✕</button>
      </div>
      <div class="drawer-grid">
        <div><strong>Time Span:</strong> {selectedSegment.startSec}s – {selectedSegment.endSec}s</div>
        <div><strong>PTS Range:</strong> {selectedSegment.startPts} – {selectedSegment.endPts}</div>
        <div><strong>Frames:</strong> {selectedSegment.transcodedFrames} / {selectedSegment.expectedFrames}</div>
        <div><strong>Assigned Worker:</strong> <span style="color: {selectedSegment.nodeColor}">{selectedSegment.assignedNode}</span></div>
        <div><strong>Encoder:</strong> <code>{selectedSegment.encoderBackend}</code></div>
        <div><strong>Worker Speed:</strong> {selectedSegment.fps.toFixed(1)} FPS</div>
      </div>
    </div>
  {/if}

  <!-- Worker Nodes Contribution Grid -->
  <section class="workers-grid">
    {#each workers as worker (worker.id)}
      <div class="worker-card" style="border-left: 4px solid {worker.color};">
        <div class="worker-header">
          <span class="worker-name">{worker.name}</span>
          <span class="worker-fps" style="color: {worker.color};">{Math.round(worker.fps)} FPS</span>
        </div>
        <div class="worker-backend"><code>{worker.backend}</code></div>
        <div class="worker-stats">
          <span>{worker.assignedSegments} GOP Segments Assigned</span>
        </div>
      </div>
    {/each}
  </section>
</div>

<style>
  .transcode-pipeline-card {
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

  .pipeline-header {
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

  .badge-job {
    background-color: #1f2937;
    color: #9ca3af;
  }

  .badge-codec {
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
    color: #ffffff;
  }

  .btn-toggle {
    background-color: #2563eb;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.5rem 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .btn-toggle:hover {
    background-color: #1d4ed8;
  }

  .metrics-strip {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1rem;
    background-color: #1f2937;
    padding: 1rem;
    border-radius: 8px;
  }

  .metric-block {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .metric-block .label {
    font-size: 0.75rem;
    color: #9ca3af;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metric-block .value {
    font-size: 1.15rem;
    font-weight: 700;
    color: #f9fafb;
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .metric-block small {
    font-size: 0.8rem;
    font-weight: normal;
    color: #9ca3af;
  }

  .speedup-highlight {
    color: #10b981 !important;
  }

  .text-success {
    color: #34d399 !important;
  }

  .text-ready {
    color: #60a5fa !important;
  }

  .timeline-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .timeline-meta {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    color: #9ca3af;
  }

  .timeline-title {
    font-weight: 600;
    color: #d1d5db;
  }

  .timeline-bar {
    display: flex;
    height: 44px;
    background-color: #1f2937;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid #374151;
  }

  .segment-segment-btn {
    position: relative;
    border: none;
    border-bottom: 4px solid transparent;
    background-color: #1f2937;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-right: 1px solid #111827;
  }

  .segment-segment-btn.selected {
    outline: 2px solid #60a5fa;
    z-index: 2;
  }

  .segment-fill {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    opacity: 0.35;
    transition: width 0.3s ease;
  }

  .segment-label {
    position: relative;
    font-size: 0.75rem;
    font-weight: 700;
    color: #f3f4f6;
    z-index: 1;
  }

  .segment-detail-drawer {
    background-color: #1f2937;
    border: 1px solid #4b5563;
    border-radius: 8px;
    padding: 1rem;
  }

  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .drawer-header h4 {
    margin: 0;
    font-size: 1rem;
    color: #60a5fa;
  }

  .btn-close {
    background: none;
    border: none;
    color: #9ca3af;
    font-size: 1.1rem;
    cursor: pointer;
  }

  .drawer-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.75rem;
    font-size: 0.85rem;
  }

  .workers-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .worker-card {
    background-color: #1f2937;
    padding: 0.9rem;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .worker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .worker-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #f3f4f6;
  }

  .worker-fps {
    font-size: 0.95rem;
    font-weight: 700;
  }

  .worker-backend code {
    font-size: 0.75rem;
    color: #9ca3af;
    background-color: #111827;
    padding: 0.15rem 0.35rem;
    border-radius: 3px;
  }

  .worker-stats {
    font-size: 0.75rem;
    color: #9ca3af;
  }
</style>
