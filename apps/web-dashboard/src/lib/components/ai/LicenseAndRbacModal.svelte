<!-- Svelte 5 License Acceptance & Model RBAC Policy Modal -->
<script lang="ts">
  interface Props {
    modelName: string;
    modelId: string;
    licenseName: string;
    licenseSummary: string;
    initialAccepted?: boolean;
    initialVisibility?: 'privateLocalOnly' | 'trustedClusterPeers' | 'publicShared';
    onSave: (accepted: boolean, visibility: 'privateLocalOnly' | 'trustedClusterPeers' | 'publicShared') => void;
    onClose: () => void;
  }

  let {
    modelName,
    modelId,
    licenseName,
    licenseSummary,
    initialAccepted = false,
    initialVisibility = 'privateLocalOnly',
    onSave,
    onClose,
  }: Props = $props();

  let agreed = $state(initialAccepted);
  let visibility = $state<'privateLocalOnly' | 'trustedClusterPeers' | 'publicShared'>(initialVisibility);
  let isSigning = $state(false);

  function handleSave() {
    isSigning = true;
    onSave(agreed, visibility);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="license-modal-title">
  <div class="modal-card">
    <div class="modal-header">
      <div>
        <h3 id="license-modal-title">Model License & RBAC Governance</h3>
        <span class="subtitle">{modelName} • {licenseName}</span>
      </div>
      <button type="button" class="btn-close" aria-label="Close" onclick={onClose}>✕</button>
    </div>

    <div class="terms-container" tabindex="0" aria-label="License agreement terms">
      <p>{licenseSummary}</p>
    </div>

    <div class="agreement-box">
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={agreed} />
        <span>I explicitly accept the <strong>{licenseName}</strong> terms and conditions on behalf of this node.</span>
      </label>
    </div>

    <div class="visibility-section">
      <h4>Cluster Visibility & Access Policy (RBAC)</h4>
      <div class="radio-option">
        <label>
          <input type="radio" value="privateLocalOnly" bind:group={visibility} />
          <strong>Private Local Only</strong>
          <small>Never broadcast over P2P mesh; remote peers receive 404 Not Found (Zero metadata leakage).</small>
        </label>
      </div>

      <div class="radio-option">
        <label>
          <input type="radio" value="trustedClusterPeers" bind:group={visibility} />
          <strong>Trusted Cluster Peers</strong>
          <small>Accessible only to peers holding valid cluster capability tokens.</small>
        </label>
      </div>

      <div class="radio-option">
        <label>
          <input type="radio" value="publicShared" bind:group={visibility} />
          <strong>Public Shared</strong>
          <small>Open for cluster-wide swarm chunk distribution and remote shard caching.</small>
        </label>
      </div>
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-cancel" onclick={onClose}>Cancel</button>
      <button
        type="button"
        class="btn-confirm"
        disabled={!agreed || isSigning}
        onclick={handleSave}
      >
        {isSigning ? 'Signing Receipt...' : 'Sign & Apply Policy'}
      </button>
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
    width: 560px;
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
    color: #4f46e5;
    font-weight: 600;
  }

  .btn-close {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
  }

  .terms-container {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    height: 100px;
    overflow-y: auto;
    padding: 10px;
    font-size: 12px;
    color: #334155;
    line-height: 1.4;
    margin-bottom: 14px;
  }

  .agreement-box {
    background: #eef2ff;
    border: 1px solid #c7d2fe;
    padding: 10px 12px;
    border-radius: 6px;
    margin-bottom: 16px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #1e1b4b;
    cursor: pointer;
  }

  .visibility-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 20px;
  }

  .visibility-section h4 {
    margin: 0 0 4px 0;
    font-size: 13px;
  }

  .radio-option label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    cursor: pointer;
  }

  .radio-option small {
    color: #64748b;
    margin-left: 20px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-cancel {
    background: none;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 8px 14px;
    font-size: 12px;
    cursor: pointer;
  }

  .btn-confirm {
    background: #4f46e5;
    color: white;
    border: none;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-confirm:disabled {
    background: #a5b4fc;
    cursor: not-allowed;
  }
</style>
