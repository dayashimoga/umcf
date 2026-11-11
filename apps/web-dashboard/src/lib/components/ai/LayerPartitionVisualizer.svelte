<script lang="ts">
  interface UiLayerStage {
    stageIndex: number;
    nodeId: string;
    deviceName: string;
    deviceType: 'Desktop' | 'Laptop' | 'Phone';
    startLayer: number;
    endLayer: number;
    hasEmbeddings: boolean;
    hasOutputHead: boolean;
    usedMemoryMb: number;
    totalMemoryMb: number;
    hopLatencyMs: number;
  }

  interface Props {
    modelName: string;
    totalLayers: number;
    isSingleNode: boolean;
    estimatedDecodeTps: number;
    estimatedTtftMs: number;
    stages: UiLayerStage[];
    onRebalance?: () => void;
  }

  let {
    modelName,
    totalLayers,
    isSingleNode,
    estimatedDecodeTps,
    estimatedTtftMs,
    stages,
    onRebalance
  }: Props = $props();

  let selectedStage = $state<number | null>(null);

  function getDeviceIcon(type: string): string {
    switch (type.toLowerCase()) {
      case 'desktop': return '🖥️';
      case 'laptop': return '💻';
      case 'phone': return '📱';
      default: return '⚙️';
    }
  }

  function getProgressColor(ratio: number): string {
    if (ratio > 0.85) return 'bg-red-500';
    if (ratio > 0.70) return 'bg-amber-500';
    return 'bg-emerald-500';
  }
</script>

<div class="bg-slate-900 border border-slate-800 rounded-2xl p-6 shadow-xl text-slate-100 max-w-4xl mx-auto">
  <!-- Header: Model Info & Inferred Metrics -->
  <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 pb-5 border-b border-slate-800">
    <div class="flex items-center gap-3">
      <div class="p-3 bg-indigo-500/10 border border-indigo-500/30 rounded-xl text-2xl">
        {isSingleNode ? '🖥️' : '🌐'}
      </div>
      <div>
        <h2 class="text-xl font-bold tracking-tight text-white">{modelName}</h2>
        <p class="text-sm text-slate-400">
          {totalLayers} Transformer Layers • {isSingleNode ? 'Single-Device Local' : `${stages.length}-Node Heterogeneous Pipeline Ring`}
        </p>
      </div>
    </div>

    <!-- Performance Telemetry Badges -->
    <div class="flex items-center gap-3 bg-slate-950/60 border border-slate-800/80 px-4 py-2.5 rounded-xl">
      <div class="text-right">
        <div class="text-xs text-slate-400">Est. Throughput</div>
        <div class="text-sm font-bold text-indigo-400">{estimatedDecodeTps.toFixed(1)} tok/s</div>
      </div>
      <div class="w-px h-7 bg-slate-800"></div>
      <div class="text-right">
        <div class="text-xs text-slate-400">Est. TTFT</div>
        <div class="text-sm font-bold text-emerald-400">{estimatedTtftMs.toFixed(0)} ms</div>
      </div>
    </div>
  </div>

  <!-- Pipeline Stages Visualizer -->
  <div class="mt-6 space-y-3">
    <div class="text-xs font-semibold uppercase tracking-wider text-slate-400">
      Pipeline Stage Allocation Ring
    </div>

    {#each stages as stage, idx}
      {@const ratio = stage.totalMemoryMb > 0 ? stage.usedMemoryMb / stage.totalMemoryMb : 0}
      {@const isSelected = selectedStage === stage.stageIndex}

      <!-- Hop Interconnect Latency Indicator -->
      {#if idx > 0}
        <div class="flex items-center gap-2 pl-8 py-1 text-xs text-slate-500 font-mono">
          <svg class="w-3.5 h-3.5 text-slate-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3" />
          </svg>
          <span>mTLS QUIC Stream Hop: {stage.hopLatencyMs.toFixed(1)} ms RTT</span>
        </div>
      {/if}

      <!-- Stage Card -->
      <button
        type="button"
        onclick={() => (selectedStage = isSelected ? null : stage.stageIndex)}
        class="w-full text-left p-4 rounded-xl border transition-all duration-200 {isSelected
          ? 'bg-slate-800/90 border-indigo-500 shadow-lg shadow-indigo-500/10'
          : 'bg-slate-950/40 border-slate-800 hover:border-slate-700 hover:bg-slate-800/30'}"
      >
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <span class="text-2xl" aria-hidden="true">{getDeviceIcon(stage.deviceType)}</span>
            <div>
              <div class="text-sm font-semibold text-white flex items-center gap-2">
                <span>{stage.deviceName}</span>
                <span class="text-xs font-mono text-slate-400">({stage.nodeId})</span>
              </div>
              <div class="flex items-center gap-2 mt-1">
                {#if stage.hasEmbeddings}
                  <span class="text-[10px] font-bold px-2 py-0.5 rounded bg-teal-500/10 text-teal-400 border border-teal-500/20">
                    Embeddings
                  </span>
                {/if}
                {#if stage.hasOutputHead}
                  <span class="text-[10px] font-bold px-2 py-0.5 rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
                    Output Head
                  </span>
                {/if}
              </div>
            </div>
          </div>

          <!-- Layer Range Pill -->
          <div class="text-right">
            <span class="inline-block px-3 py-1 bg-indigo-500/10 text-indigo-300 border border-indigo-500/20 rounded-full text-xs font-semibold">
              Layers {stage.startLayer}–{stage.endLayer} ({stage.endLayer - stage.startLayer + 1} layers)
            </span>
            <div class="text-xs text-slate-400 mt-1">
              {(stage.usedMemoryMb / 1024).toFixed(1)} / {(stage.totalMemoryMb / 1024).toFixed(1)} GB VRAM
            </div>
          </div>
        </div>

        <!-- Memory Gauge Bar -->
        <div class="mt-3 w-full bg-slate-800 h-1.5 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-300 {getProgressColor(ratio)}"
            style="width: {(ratio * 100).toFixed(1)}%"
          ></div>
        </div>
      </button>
    {/each}
  </div>

  <!-- Rebalance Action Trigger -->
  <div class="mt-6 pt-4 border-t border-slate-800 flex justify-end">
    <button
      type="button"
      onclick={onRebalance}
      class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors flex items-center gap-2"
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      <span>Rebalance Layer Partitioning</span>
    </button>
  </div>
</div>

