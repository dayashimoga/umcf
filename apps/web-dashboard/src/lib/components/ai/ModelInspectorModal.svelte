<!-- Svelte 5 Model Inspector Modal & KV-Cache / Memory Footprint Calculator -->
<script lang="ts">
  interface ModelHyperparameters {
    architecture: string;
    blockCount: number;
    defaultContextLength: number;
    embeddingLength: number;
    headCount: number;
    headCountKv: number;
    totalWeightBytes: number;
    primaryQuantization: string;
  }

  interface Props {
    filename: string;
    modelId: string;
    params: ModelHyperparameters;
    deviceAvailableRamBytes?: number;
    onClose: () => void;
  }

  let {
    filename,
    modelId,
    params,
    deviceAvailableRamBytes = 8 * 1024 * 1024 * 1024, // 8 GiB default
    onClose,
  }: Props = $props();

  let contextLength = $state(params.defaultContextLength);
  let batchSize = $state(512);
  let flashAttention = $state(true);

  // Exact GQA KV-cache bytes: 2 * layers * ctx * (embd / heads * heads_kv) * 2 (F16)
  let kvCacheBytes = $derived.by(() => {
    const headDim = Math.floor(params.embeddingLength / Math.max(1, params.headCount));
    const nEmbdGqa = headDim * params.headCountKv;
    return 2 * params.blockCount * contextLength * nEmbdGqa * 2;
  });

  // Compute scratch: FFN + self attention + base runtime overhead
  let computeScratchBytes = $derived.by(() => {
    const ffnLen = params.embeddingLength * 4;
    const ffnScratch = ffnLen * batchSize * 4;
    const headDim = Math.floor(params.embeddingLength / Math.max(1, params.headCount));
    const attnScratch = flashAttention
      ? params.headCount * headDim * batchSize * 4
      : params.headCount * Math.floor(contextLength / 4) * batchSize * 4;
    return ffnScratch + attnScratch + (48 * 1024 * 1024);
  });

  let totalMemoryRequired = $derived(
    params.totalWeightBytes + kvCacheBytes + computeScratchBytes
  );

  let utilizationRatio = $derived(
    totalMemoryRequired / Math.max(1, deviceAvailableRamBytes)
  );

  function formatBytes(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    if (mb < 1024) return `${mb.toFixed(1)} MiB`;
    return `${(mb / 1024).toFixed(2)} GiB`;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="modal-title">
  <div class="modal-card">
    <div class="modal-header">
      <div>
        <h2 id="modal-title">{filename}</h2>
        <span class="subtitle">{params.architecture.toUpperCase()} • {params.primaryQuantization} • ID: {modelId.slice(0, 16)}...</span>
      </div>
      <button type="button" class="btn-close" aria-label="Close modal" onclick={onClose}>✕</button>
    </div>

    <div class="params-grid">
      <div class="param-box">
        <span class="label">Layers</span>
        <strong>{params.blockCount}</strong>
      </div>
      <div class="param-box">
        <span class="label">Embedding</span>
        <strong>{params.embeddingLength}</strong>
      </div>
      <div class="param-box">
        <span class="label">Attention Heads</span>
        <strong>{params.headCount} (Q) / {params.headCountKv} (KV)</strong>
      </div>
      <div class="param-box">
        <span class="label">Weight Footprint</span>
        <strong>{formatBytes(params.totalWeightBytes)}</strong>
      </div>
    </div>

    <div class="calculator-section">
      <h3>Interactive Memory Footprint Calculator</h3>

      <div class="input-row">
        <label for="ctx-input">
          Context Window: <strong>{contextLength.toLocaleString()} tokens</strong>
        </label>
        <input
          id="ctx-input"
          type="range"
          min="512"
          max="65536"
          step="512"
          bind:value={contextLength}
        />
      </div>

      <div class="input-row">
        <label for="batch-input">
          Batch Size: <strong>{batchSize}</strong>
        </label>
        <input
          id="batch-input"
          type="range"
          min="64"
          max="2048"
          step="64"
          bind:value={batchSize}
        />
      </div>

      <div class="checkbox-row">
        <label>
          <input type="checkbox" bind:checked={flashAttention} />
          Enable Flash Attention (reduces intermediate attention scratch buffer)
        </label>
      </div>
    </div>

    <div class="memory-bar-section">
      <div class="bar-header">
        <span>Estimated Memory Allocation</span>
        <strong>{formatBytes(totalMemoryRequired)} / {formatBytes(deviceAvailableRamBytes)}</strong>
      </div>

      <div class="progress-bar-track">
        <div
          class="progress-bar-fill {utilizationRatio > 0.95 ? 'danger' : utilizationRatio > 0.70 ? 'warning' : 'safe'}"
          style="width: {Math.min(100, Math.round(utilizationRatio * 100))}%;"
        ></div>
      </div>

      <div class="verdict-banner {utilizationRatio > 0.95 ? 'danger' : utilizationRatio > 0.70 ? 'warning' : 'safe'}">
        {#if utilizationRatio <= 0.70}
          ✓ Fits Single Node with High Headroom (Full GPU Offload Feasible)
        {:else if utilizationRatio <= 0.95}
          ⚠ Fits Single Node with Tight Headroom (Partial Context Recommended)
        {:else}
          ✕ Insufficient Memory: Requires Cluster Split ({Math.ceil(utilizationRatio)} Nodes Needed)
        {/if}
      </div>

      <table class="breakdown-table">
        <tbody>
          <tr>
            <td>Static Model Weights</td>
            <td class="text-right">{formatBytes(params.totalWeightBytes)}</td>
          </tr>
          <tr>
            <td>KV-Cache ({params.primaryQuantization})</td>
            <td class="text-right">{formatBytes(kvCacheBytes)}</td>
          </tr>
          <tr>
            <td>Compute Scratch Buffer</td>
            <td class="text-right">{formatBytes(computeScratchBytes)}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-primary" onclick={onClose}>Done</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }

  .modal-card {
    background: #ffffff;
    border-radius: 12px;
    width: 600px;
    max-width: 90vw;
    padding: 24px;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }

  .subtitle {
    font-size: 12px;
    color: #64748b;
  }

  .btn-close {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
  }

  .params-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin-bottom: 20px;
  }

  .param-box {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    padding: 8px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
  }

  .param-box .label {
    font-size: 10px;
    color: #64748b;
    text-transform: uppercase;
  }

  .calculator-section {
    background: #f1f5f9;
    padding: 14px;
    border-radius: 8px;
    margin-bottom: 16px;
  }

  .input-row {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-row input {
    width: 100%;
  }

  .checkbox-row {
    margin-top: 10px;
    font-size: 12px;
  }

  .progress-bar-track {
    height: 10px;
    background: #e2e8f0;
    border-radius: 5px;
    overflow: hidden;
    margin: 8px 0;
  }

  .progress-bar-fill {
    height: 100%;
    transition: width 0.2s ease;
  }

  .progress-bar-fill.safe { background: #10b981; }
  .progress-bar-fill.warning { background: #f59e0b; }
  .progress-bar-fill.danger { background: #ef4444; }

  .verdict-banner {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 12px;
  }

  .verdict-banner.safe { background: #d1fae5; color: #065f46; }
  .verdict-banner.warning { background: #fef3c7; color: #92400e; }
  .verdict-banner.danger { background: #fee2e2; color: #991b1b; }

  .breakdown-table {
    width: 100%;
    font-size: 12px;
    border-collapse: collapse;
  }

  .breakdown-table td {
    padding: 4px 0;
    border-bottom: 1px solid #f1f5f9;
  }

  .text-right {
    text-align: right;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 20px;
  }

  .btn-primary {
    background: #4f46e5;
    color: white;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
</style>

