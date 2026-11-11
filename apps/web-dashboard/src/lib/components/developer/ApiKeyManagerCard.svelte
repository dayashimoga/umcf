<!--
  ApiKeyManagerCard.svelte
  UMCF Stage 07 Phase 2 Subphase 1 (S7.P2.SP1)
  Scoped Capability Token Management, Rate Limiting & Developer API Gateway Explorer.
  Pure Svelte 5 runes ($state, $derived), strict TypeScript, and WCAG 2.1 AA accessibility.
-->
<script lang="ts">
  type ScopeName =
    | "cluster:read"
    | "cluster:admin"
    | "jobs:submit"
    | "jobs:read"
    | "ai:inference"
    | "gpu:render"
    | "media:transcode"
    | "storage:read"
    | "storage:write"
    | "stream:connect"
    | "admin:full";

  interface ApiKeyItem {
    id: string;
    label: string;
    scopes: ScopeName[];
    rateLimitRps: number;
    burst: number;
    createdAt: string;
    expiresAt?: string;
    revoked: boolean;
  }

  const ALL_SCOPES: Array<{ scope: ScopeName; description: string }> = [
    { scope: "cluster:read", description: "Query cluster topology, nodes, and telemetry" },
    { scope: "cluster:admin", description: "Pair new devices and manage cluster membership" },
    { scope: "jobs:submit", description: "Submit new compute, WASM, or distributed tasks" },
    { scope: "jobs:read", description: "Query job status, logs, and progress events" },
    { scope: "ai:inference", description: "Execute LLM chat, prompts, and embeddings" },
    { scope: "gpu:render", description: "Dispatch tile/frame GPU rendering jobs" },
    { scope: "media:transcode", description: "Submit video/audio transcoding workloads" },
    { scope: "storage:read", description: "Read content-addressed chunks and objects" },
    { scope: "storage:write", description: "Ingest and pin objects for offline access" },
    { scope: "stream:connect", description: "Connect to low-latency remote desktop streaming" },
    { scope: "admin:full", description: "Unrestricted administrative superuser access" },
  ];

  // --- Reactive Runes State ---
  let keys = $state<ApiKeyItem[]>([
    {
      id: "key_live_9a8b1c2d",
      label: "Production Inference Service",
      scopes: ["ai:inference", "storage:read", "jobs:read"],
      rateLimitRps: 50,
      burst: 100,
      createdAt: "2026-11-10",
      expiresAt: "2027-11-10",
      revoked: false,
    },
    {
      id: "key_live_3f4e5d6c",
      label: "CI/CD Pipeline Runner",
      scopes: ["jobs:submit", "jobs:read", "storage:write", "storage:read"],
      rateLimitRps: 20,
      burst: 40,
      createdAt: "2026-11-08",
      expiresAt: "2027-02-08",
      revoked: false,
    },
    {
      id: "key_live_7b8c9d0e",
      label: "Cluster Admin Superuser",
      scopes: ["admin:full"],
      rateLimitRps: 200,
      burst: 500,
      createdAt: "2026-11-01",
      revoked: false,
    },
  ]);

  let isCreating = $state(false);
  let newKeyLabel = $state("");
  let selectedScopes = $state<ScopeName[]>(["cluster:read", "jobs:read"]);
  let newKeyRps = $state(50);
  let newKeyBurst = $state(100);
  let newlyGeneratedToken = $state<string | null>(null);
  let copyFeedback = $state(false);

  // --- Derived Metrics ---
  const activeKeysCount = $derived(keys.filter((k) => !k.revoked).length);
  const totalKeysCount = $derived(keys.length);

  // --- Interactive Actions ---
  function openCreateModal() {
    newKeyLabel = "";
    selectedScopes = ["cluster:read", "jobs:read"];
    newKeyRps = 50;
    newKeyBurst = 100;
    newlyGeneratedToken = null;
    isCreating = true;
  }

  function toggleScope(scope: ScopeName) {
    if (scope === "admin:full") {
      selectedScopes = selectedScopes.includes("admin:full") ? [] : ["admin:full"];
      return;
    }
    if (selectedScopes.includes("admin:full")) {
      selectedScopes = [scope];
      return;
    }
    if (selectedScopes.includes(scope)) {
      selectedScopes = selectedScopes.filter((s) => s !== scope);
    } else {
      selectedScopes = [...selectedScopes, scope];
    }
  }

  function createKey() {
    if (!newKeyLabel.trim()) return;

    const hexBytes = Array.from({ length: 24 }, () =>
      Math.floor(Math.random() * 256).toString(16).padStart(2, "0")
    ).join("");

    const token = `umcf_live_${hexBytes}`;
    const keyId = `key_live_${hexBytes.slice(0, 8)}`;

    keys.push({
      id: keyId,
      label: newKeyLabel.trim(),
      scopes: selectedScopes.length ? selectedScopes : ["cluster:read"],
      rateLimitRps: newKeyRps,
      burst: newKeyBurst,
      createdAt: new Date().toISOString().slice(0, 10),
      expiresAt: "2027-11-11",
      revoked: false,
    });

    newlyGeneratedToken = token;
  }

  function revokeKey(id: string) {
    const target = keys.find((k) => k.id === id);
    if (target) {
      target.revoked = true;
    }
  }

  function copyToken() {
    if (!newlyGeneratedToken) return;
    navigator.clipboard.writeText(newlyGeneratedToken);
    copyFeedback = true;
    setTimeout(() => {
      copyFeedback = false;
    }, 2000);
  }
</script>

<div class="card" role="region" aria-label="API Key Management and Developer Gateway">
  <div class="header">
    <div class="title-group">
      <div class="icon-badge" aria-hidden="true">🔑</div>
      <div>
        <h2>Developer API Keys & Capabilities</h2>
        <p class="subtitle">
          Manage fine-grained Bearer capability tokens for Rust, TypeScript, and Python SDK clients.
        </p>
      </div>
    </div>
    <div class="actions">
      <a href="/api/v1/docs" target="_blank" rel="noreferrer" class="btn btn-secondary">
        Interactive API Docs &rarr;
      </a>
      <button class="btn btn-primary" onclick={openCreateModal} aria-haspopup="dialog">
        + Create API Key
      </button>
    </div>
  </div>

  <!-- Key Metrics Row -->
  <div class="metrics-grid">
    <div class="metric-item">
      <span class="metric-label">Active API Keys</span>
      <span class="metric-value">{activeKeysCount} / {totalKeysCount}</span>
    </div>
    <div class="metric-item">
      <span class="metric-label">API Gateway Root</span>
      <span class="metric-value code">/api/v1/*</span>
    </div>
    <div class="metric-item">
      <span class="metric-label">Event Stream (WebSocket)</span>
      <span class="metric-value code">/ws/v1/jobs/*</span>
    </div>
    <div class="metric-item">
      <span class="metric-label">Error Standard</span>
      <span class="metric-value">RFC 9457 Problem Details</span>
    </div>
  </div>

  <!-- Table of Keys -->
  <div class="table-container">
    <table class="keys-table" aria-label="Active API Keys">
      <thead>
        <tr>
          <th scope="col">Key Label</th>
          <th scope="col">Key Identifier</th>
          <th scope="col">Capability Scopes</th>
          <th scope="col">Rate Limit</th>
          <th scope="col">Status</th>
          <th scope="col">Actions</th>
        </tr>
      </thead>
      <tbody>
        {#each keys as key (key.id)}
          <tr class:revoked-row={key.revoked}>
            <td class="font-bold">{key.label}</td>
            <td class="font-mono text-muted">{key.id}</td>
            <td>
              <div class="scope-badges">
                {#each key.scopes as scope}
                  <span
                    class="scope-chip"
                    class:chip-admin={scope === "admin:full"}
                    class:chip-ai={scope === "ai:inference"}
                    class:chip-storage={scope.startsWith("storage:")}
                  >
                    {scope}
                  </span>
                {/each}
              </div>
            </td>
            <td class="text-muted">
              {key.rateLimitRps} rps (burst: {key.burst})
            </td>
            <td>
              {#if key.revoked}
                <span class="status-badge status-revoked">Revoked</span>
              {:else}
                <span class="status-badge status-active">Active</span>
              {/if}
            </td>
            <td>
              {#if !key.revoked}
                <button
                  class="btn-revoke"
                  onclick={() => revokeKey(key.id)}
                  aria-label="Revoke key {key.label}"
                >
                  Revoke
                </button>
              {:else}
                <span class="text-muted text-sm">—</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<!-- Create Modal Dialog -->
{#if isCreating}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="modal-title">
    <div class="modal-card">
      <h3 id="modal-title">Create Scoped Capability API Key</h3>

      {#if newlyGeneratedToken}
        <div class="alert-success" role="alert">
          <strong>Key Created Successfully!</strong>
          <p class="text-sm">
            Please copy this secret Bearer token now. For your security, it will not be displayed again.
          </p>
          <div class="token-copy-box">
            <code class="token-text">{newlyGeneratedToken}</code>
            <button class="btn btn-secondary" onclick={copyToken}>
              {copyFeedback ? "Copied! ✓" : "Copy Token"}
            </button>
          </div>
          <button class="btn btn-primary w-full mt-4" onclick={() => (isCreating = false)}>
            Done
          </button>
        </div>
      {:else}
        <div class="form-group">
          <label for="key-label">Key Descriptive Label</label>
          <input
            id="key-label"
            type="text"
            bind:value={newKeyLabel}
            placeholder="e.g. Production AI Cluster Service"
            class="input-text"
          />
        </div>

        <div class="form-group">
          <span class="field-title">Granted Capability Scopes</span>
          <div class="scopes-selection-grid">
            {#each ALL_SCOPES as item}
              <button
                type="button"
                class="scope-select-btn"
                class:selected={selectedScopes.includes(item.scope)}
                onclick={() => toggleScope(item.scope)}
              >
                <span class="scope-name">{item.scope}</span>
                <span class="scope-desc">{item.description}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="rate-rps">Sustained Limit (RPS)</label>
            <input
              id="rate-rps"
              type="number"
              bind:value={newKeyRps}
              min="1"
              max="10000"
              class="input-text"
            />
          </div>
          <div class="form-group">
            <label for="burst-limit">Burst Capacity</label>
            <input
              id="burst-limit"
              type="number"
              bind:value={newKeyBurst}
              min="1"
              max="20000"
              class="input-text"
            />
          </div>
        </div>

        <div class="modal-actions">
          <button class="btn btn-secondary" onclick={() => (isCreating = false)}>
            Cancel
          </button>
          <button class="btn btn-primary" onclick={createKey} disabled={!newKeyLabel.trim()}>
            Generate Secret Token
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 12px;
    padding: 24px;
    color: #f8fafc;
  }
  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 24px;
  }
  .title-group {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .icon-badge {
    font-size: 28px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
  }
  .subtitle {
    font-size: 13px;
    color: #94a3b8;
    margin-top: 4px;
  }
  .actions {
    display: flex;
    gap: 12px;
  }
  .btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background 0.15s ease;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
  }
  .btn-primary {
    background: #0284c7;
    color: white;
  }
  .btn-primary:hover:not(:disabled) {
    background: #0369a1;
  }
  .btn-secondary {
    background: #1e293b;
    border: 1px solid #334155;
    color: #38bdf8;
  }
  .btn-secondary:hover {
    background: #334155;
  }
  .btn-revoke {
    background: transparent;
    border: 1px solid #ef4444;
    color: #ef4444;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-revoke:hover {
    background: rgba(239, 68, 68, 0.15);
  }
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
    margin-bottom: 24px;
  }
  .metric-item {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .metric-label {
    font-size: 11px;
    color: #94a3b8;
    text-transform: uppercase;
  }
  .metric-value {
    font-size: 15px;
    font-weight: 700;
    color: #38bdf8;
  }
  .code {
    font-family: monospace;
  }
  .table-container {
    overflow-x: auto;
  }
  .keys-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 13px;
  }
  .keys-table th {
    padding: 10px 12px;
    border-bottom: 1px solid #334155;
    color: #94a3b8;
    font-size: 11px;
    text-transform: uppercase;
  }
  .keys-table td {
    padding: 14px 12px;
    border-bottom: 1px solid #1e293b;
    vertical-align: middle;
  }
  .revoked-row td {
    opacity: 0.5;
  }
  .font-bold {
    font-weight: 600;
  }
  .font-mono {
    font-family: monospace;
  }
  .text-muted {
    color: #94a3b8;
  }
  .scope-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .scope-chip {
    font-size: 11px;
    font-family: monospace;
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(56, 189, 248, 0.3);
  }
  .chip-admin {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.4);
  }
  .chip-ai {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border-color: rgba(168, 85, 247, 0.4);
  }
  .chip-storage {
    background: rgba(74, 222, 128, 0.15);
    color: #4ade80;
    border-color: rgba(74, 222, 128, 0.4);
  }
  .status-badge {
    font-size: 11px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 12px;
  }
  .status-active {
    background: rgba(74, 222, 128, 0.15);
    color: #4ade80;
  }
  .status-revoked {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }
  /* Modal Styles */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    z-index: 1000;
  }
  .modal-card {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 12px;
    max-width: 640px;
    width: 100%;
    padding: 24px;
    max-height: 90vh;
    overflow-y: auto;
  }
  .form-group {
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-row {
    display: flex;
    gap: 16px;
  }
  .form-row > * {
    flex: 1;
  }
  .input-text {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    color: #f8fafc;
    padding: 8px 12px;
    font-size: 13px;
  }
  .scopes-selection-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
  }
  .scope-select-btn {
    text-align: left;
    padding: 8px;
    border-radius: 6px;
    border: 1px solid #334155;
    background: #0f172a;
    color: #94a3b8;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .scope-select-btn.selected {
    border-color: #38bdf8;
    background: rgba(56, 189, 248, 0.15);
    color: #f8fafc;
  }
  .scope-name {
    font-size: 11px;
    font-family: monospace;
    font-weight: 700;
    color: #38bdf8;
  }
  .scope-desc {
    font-size: 10px;
    color: #94a3b8;
    line-height: 1.2;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 24px;
  }
  .token-copy-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #090d16;
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid #334155;
    margin-top: 12px;
  }
  .token-text {
    font-family: monospace;
    font-size: 13px;
    color: #4ade80;
    flex: 1;
    overflow-x: auto;
  }
  .alert-success {
    background: rgba(74, 222, 128, 0.1);
    border: 1px solid rgba(74, 222, 128, 0.3);
    border-radius: 8px;
    padding: 16px;
    margin-top: 12px;
  }
  .w-full {
    width: 100%;
  }
  .mt-4 {
    margin-top: 16px;
  }
</style>

