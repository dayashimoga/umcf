<!-- Svelte 5 Interactive llama.cpp Inference Playground Modal -->
<script lang="ts">
  interface BackendInfo {
    id: string;
    name: string;
    type: 'cpu' | 'gpu' | 'accel';
    memoryFreeBytes: number;
    memoryTotalBytes: number;
  }

  interface Props {
    modelName: string;
    modelId: string;
    backends?: BackendInfo[];
    onClose: () => void;
    onGenerate?: (prompt: string, backend: string, temp: number, topK: number, onToken: (chunk: string) => void) => Promise<void>;
    onAbort?: () => void;
  }

  let {
    modelName,
    modelId,
    backends = [
      { id: 'cpu', name: 'Host CPU (AVX2/NEON)', type: 'cpu', memoryFreeBytes: 16 * 1024 * 1024 * 1024, memoryTotalBytes: 32 * 1024 * 1024 * 1024 },
      { id: 'vulkan', name: 'Vulkan0 (Discrete GPU)', type: 'gpu', memoryFreeBytes: 8 * 1024 * 1024 * 1024, memoryTotalBytes: 12 * 1024 * 1024 * 1024 },
    ],
    onClose,
    onGenerate,
    onAbort,
  }: Props = $props();

  let selectedBackend = $state(backends[0]?.id ?? 'cpu');
  let prompt = $state('Explain how distributed llama.cpp pipeline parallelism works.');
  let temperature = $state(0.7);
  let topK = $state(40);
  let useGreedy = $state(false);

  let isGenerating = $state(false);
  let generatedOutput = $state('');
  let tokenCount = $state(0);
  let tokensPerSecond = $state(0.0);
  let startTime = $state<number | null>(null);

  function formatBytes(bytes: number): string {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  }

  async function handleStartGeneration() {
    if (!prompt.trim() || isGenerating) return;

    isGenerating = true;
    generatedOutput = '';
    tokenCount = 0;
    tokensPerSecond = 0.0;
    startTime = performance.now();

    const onToken = (chunk: string) => {
      generatedOutput += chunk;
      tokenCount++;
      if (startTime) {
        const elapsedSec = (performance.now() - startTime) / 1000.0;
        if (elapsedSec > 0) {
          tokensPerSecond = Number((tokenCount / elapsedSec).toFixed(1));
        }
      }
    };

    if (onGenerate) {
      try {
        await onGenerate(prompt, selectedBackend, useGreedy ? 0.0 : temperature, useGreedy ? 1 : topK, onToken);
      } catch (err) {
        console.error('Inference error:', err);
      } finally {
        isGenerating = false;
      }
    } else {
      // Mock streaming for preview
      const chunks = [
        ' Distributed', ' pipeline', ' parallelism', ' partitions', ' transformer',
        ' layers', ' sequentially', ' across', ' heterogeneous', ' devices', '.',
        ' Activations', ' flow', ' over', ' authenticated', ' QUIC', ' streams',
        ' while', ' intermediate', ' KV', ' caches', ' remain', ' local', '.'
      ];
      for (const c of chunks) {
        if (!isGenerating) break;
        await new Promise((r) => setTimeout(r, 60));
        onToken(c);
      }
      isGenerating = false;
    }
  }

  function handleStopGeneration() {
    onAbort?.();
    isGenerating = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (isGenerating) {
        handleStopGeneration();
      } else {
        onClose();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="playground-title">
  <div class="modal-card">
    <div class="modal-header">
      <div>
        <h3 id="playground-title">AI Inference Playground</h3>
        <span class="subtitle">{modelName} • {modelId.slice(0, 16)}...</span>
      </div>
      <div class="header-right">
        <span class="tps-badge" class:active={isGenerating}>
          {isGenerating ? `${tokensPerSecond} T/s` : 'Idle'}
        </span>
        <button type="button" class="btn-close" aria-label="Close" onclick={onClose}>✕</button>
      </div>
    </div>

    <div class="modal-body">
      <!-- Hardware Backend Selector -->
      <div class="form-group">
        <label for="backend-select">Execution Backend</label>
        <select id="backend-select" bind:value={selectedBackend} disabled={isGenerating}>
          {#each backends as b}
            <option value={b.id}>
              {b.name} ({b.type.toUpperCase()}) — Free: {formatBytes(b.memoryFreeBytes)} / Total: {formatBytes(b.memoryTotalBytes)}
            </option>
          {/each}
        </select>
      </div>

      <!-- Sampler Controls -->
      <div class="sampler-row">
        <div class="sampler-toggle">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={useGreedy} disabled={isGenerating} />
            <span>Greedy (ArgMax)</span>
          </label>
        </div>

        {#if !useGreedy}
          <div class="sampler-slider">
            <label for="temp-slider">Temperature: {temperature.toFixed(2)}</label>
            <input id="temp-slider" type="range" min="0.1" max="2.0" step="0.05" bind:value={temperature} disabled={isGenerating} />
          </div>
          <div class="sampler-slider">
            <label for="topk-slider">Top-K: {topK}</label>
            <input id="topk-slider" type="range" min="1" max="100" step="1" bind:value={topK} disabled={isGenerating} />
          </div>
        {/if}
      </div>

      <!-- Prompt Input -->
      <div class="form-group">
        <label for="prompt-area">Inference Prompt</label>
        <textarea id="prompt-area" rows="3" bind:value={prompt} disabled={isGenerating}></textarea>
      </div>

      <!-- Actions -->
      <div class="action-row">
        {#if isGenerating}
          <button type="button" class="btn-stop" onclick={handleStopGeneration}>
            ⏹ Stop Generation
          </button>
        {:else}
          <button type="button" class="btn-generate" onclick={handleStartGeneration}>
            ▶ Generate Tokens
          </button>
        {/if}
      </div>

      <!-- Live Generated Output Screen -->
      <div class="output-panel">
        <div class="output-header">
          <span>Streaming Response</span>
          <span class="token-count">Tokens: {tokenCount}</span>
        </div>
        <pre class="output-content">{generatedOutput || 'Waiting for prompt generation...'}</pre>
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .modal-card {
    background: #1e1e24;
    color: #e2e8f0;
    width: 90%;
    max-width: 720px;
    border-radius: 8px;
    border: 1px solid #334155;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #334155;
  }
  .subtitle {
    font-size: 0.85rem;
    color: #94a3b8;
  }
  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tps-badge {
    padding: 4px 10px;
    border-radius: 9999px;
    font-size: 0.85rem;
    font-weight: 600;
    background: #334155;
    color: #94a3b8;
  }
  .tps-badge.active {
    background: #065f46;
    color: #34d399;
  }
  .btn-close {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 1.25rem;
    cursor: pointer;
  }
  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  select, textarea {
    background: #0f172a;
    color: #f8fafc;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 10px;
    font-family: inherit;
  }
  .sampler-row {
    display: flex;
    gap: 16px;
    align-items: center;
    background: #0f172a;
    padding: 12px;
    border-radius: 6px;
    border: 1px solid #334155;
  }
  .sampler-slider {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    font-size: 0.85rem;
  }
  .action-row {
    display: flex;
  }
  .btn-generate {
    flex: 1;
    padding: 12px;
    border: none;
    border-radius: 6px;
    background: #2563eb;
    color: white;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-stop {
    flex: 1;
    padding: 12px;
    border: none;
    border-radius: 6px;
    background: #dc2626;
    color: white;
    font-weight: 600;
    cursor: pointer;
  }
  .output-panel {
    background: #090d16;
    border: 1px solid #1e293b;
    border-radius: 6px;
    padding: 14px;
  }
  .output-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
    color: #94a3b8;
    margin-bottom: 8px;
    font-weight: 600;
  }
  .output-content {
    margin: 0;
    font-family: monospace;
    font-size: 0.9rem;
    color: #f1f5f9;
    white-space: pre-wrap;
    min-height: 80px;
  }
</style>
