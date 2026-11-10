<!-- Svelte 5 Shard Distribution Table & Heatmap Component -->
<script lang="ts">
  interface ModelShardRow {
    modelId: string;
    filename: string;
    totalSizeBytes: number;
    downloadedBytes: number;
    verifiedChunks: number;
    totalChunks: number;
    bitfield: boolean[];
    activePeers: number;
    status: 'idle' | 'downloading' | 'paused' | 'completed' | 'failed';
    bytesPerSec: number;
  }

  interface Props {
    models: ModelShardRow[];
    onPause: (modelId: string) => void;
    onResume: (modelId: string) => void;
    onCancel: (modelId: string) => void;
    onEvict: (modelId: string) => void;
  }

  let { models, onPause, onResume, onCancel, onEvict }: Props = $props();

  let searchQuery = $state('');

  let filteredModels = $derived(
    models.filter(m =>
      m.filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.modelId.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const mb = bytes / (1024 * 1024);
    if (mb < 1024) return `${mb.toFixed(1)} MiB`;
    return `${(mb / 1024).toFixed(2)} GiB`;
  }

  function formatSpeed(bps: number): string {
    const mbps = bps / (1024 * 1024);
    return `${mbps.toFixed(1)} MiB/s`;
  }
</script>

<div class="shard-table-card">
  <div class="header-row">
    <h3>Cluster Model Shard Distribution</h3>
    <input
      type="search"
      bind:value={searchQuery}
      placeholder="Filter models..."
      class="search-input"
    />
  </div>

  <table class="table" aria-label="Model Shard Distribution">
    <thead>
      <tr>
        <th scope="col">Model / Filename</th>
        <th scope="col">Size</th>
        <th scope="col">Status</th>
        <th scope="col">Progress</th>
        <th scope="col">Bitfield Heatmap</th>
        <th scope="col">Swarm Peers</th>
        <th scope="col">Speed</th>
        <th scope="col">Actions</th>
      </tr>
    </thead>
    <tbody>
      {#if filteredModels.length === 0}
        <tr>
          <td colspan="8" class="empty-state">No models registered or downloading</td>
        </tr>
      {:else}
        {#each filteredModels as model (model.modelId)}
          {@const ratio = model.totalSizeBytes > 0 ? (model.downloadedBytes / model.totalSizeBytes) : 0}
          <tr>
            <td class="name-cell">
              <strong>{model.filename}</strong>
              <span class="mono-sub">{model.modelId.slice(0, 16)}...</span>
            </td>
            <td>{formatBytes(model.totalSizeBytes)}</td>
            <td>
              <span class="status-badge {model.status}">{model.status.toUpperCase()}</span>
            </td>
            <td>
              <div class="progress-bar-container">
                <div class="progress-bar-fill" style="width: {Math.round(ratio * 100)}%;"></div>
              </div>
              <small>{(ratio * 100).toFixed(1)}% ({model.verifiedChunks}/{model.totalChunks})</small>
            </td>
            <td>
              <div class="heatmap-strip" title="{model.verifiedChunks} of {model.totalChunks} chunks verified">
                {#each model.bitfield.slice(0, 32) as isSet}
                  <span class="chunk-pixel {isSet ? 'verified' : 'pending'}"></span>
                {/each}
              </div>
            </td>
            <td>{model.activePeers} peers</td>
            <td>{formatSpeed(model.bytesPerSec)}</td>
            <td class="actions-cell">
              {#if model.status === 'downloading'}
                <button
                  type="button"
                  class="btn-action pause"
                  aria-label="Pause download"
                  onclick={() => onPause(model.modelId)}
                >
                  Pause
                </button>
              {:else if model.status === 'paused'}
                <button
                  type="button"
                  class="btn-action resume"
                  aria-label="Resume download"
                  onclick={() => onResume(model.modelId)}
                >
                  Resume
                </button>
              {/if}

              {#if model.status !== 'completed'}
                <button
                  type="button"
                  class="btn-action cancel"
                  aria-label="Cancel download"
                  onclick={() => onCancel(model.modelId)}
                >
                  Cancel
                </button>
              {:else}
                <button
                  type="button"
                  class="btn-action evict"
                  aria-label="Evict model"
                  onclick={() => onEvict(model.modelId)}
                >
                  Evict
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>

<style>
  .shard-table-card {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .search-input {
    padding: 6px 12px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 13px;
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th, td {
    padding: 10px 12px;
    text-align: left;
    border-bottom: 1px solid #f1f5f9;
  }

  th {
    font-weight: 600;
    color: #475569;
  }

  .name-cell {
    display: flex;
    flex-direction: column;
  }

  .mono-sub {
    font-family: monospace;
    font-size: 11px;
    color: #64748b;
  }

  .status-badge {
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: bold;
    color: white;
  }

  .status-badge.downloading { background: #10b981; }
  .status-badge.paused { background: #f59e0b; }
  .status-badge.completed { background: #3b82f6; }
  .status-badge.failed { background: #ef4444; }
  .status-badge.idle { background: #94a3b8; }

  .progress-bar-container {
    width: 100px;
    height: 6px;
    background: #e2e8f0;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 4px;
  }

  .progress-bar-fill {
    height: 100%;
    background: #3b82f6;
  }

  .heatmap-strip {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: #f8fafc;
    border-radius: 2px;
    width: fit-content;
  }

  .chunk-pixel {
    width: 5px;
    height: 8px;
    border-radius: 1px;
  }

  .chunk-pixel.verified { background: #10b981; }
  .chunk-pixel.pending { background: #cbd5e1; }

  .actions-cell {
    display: flex;
    gap: 6px;
  }

  .btn-action {
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
  }

  .btn-action.pause { background: #fef3c7; color: #92400e; }
  .btn-action.resume { background: #dbeafe; color: #1e40af; }
  .btn-action.cancel { background: #fee2e2; color: #991b1b; }
  .btn-action.evict { background: #f1f5f9; color: #475569; }

  .empty-state {
    text-align: center;
    color: #94a3b8;
    padding: 24px;
  }
</style>

