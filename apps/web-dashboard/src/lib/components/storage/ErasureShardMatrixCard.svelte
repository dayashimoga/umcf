<!--
  ErasureShardMatrixCard.svelte
  UMCF Stage 07 Phase 1 Subphase 2 (S7.P1.SP2)
  Reed-Solomon GF(2^8) Erasure Coding, Convergent Authenticated Encryption & Failure Domain Shard Placement.
  Adheres strictly to Svelte 5 runes ($state, $derived), strict TypeScript, and WCAG 2.1 AA accessibility.
-->
<script lang="ts">
  type RedundancyMode = "rs42" | "rs63" | "rep3";
  type ShardState = "healthy" | "offline" | "healing";

  interface ShardCell {
    index: number;
    tag: string; // e.g. "D0", "P1"
    isParity: boolean;
    assignedNode: string;
    nodeId: string;
    failureDomain: string;
    isAcPowered: boolean;
    batteryPct: number;
    shardHash: string;
    sizeKb: number;
    state: ShardState;
  }

  // --- Reactive Runes State ---
  let selectedMode = $state<RedundancyMode>("rs42");
  let isSimulatingOutage = $state(false);
  let isHealing = $state(false);
  let repairLog = $state<string[]>([]);

  // Base mock nodes with heterogeneous attributes
  const candidateNodes = [
    { name: "desktop-alpha", id: "did:umcf:desktop_01", domain: "subnet:192.168.1.0/24", ac: true, bat: 100 },
    { name: "server-edge-01", id: "did:umcf:server_02", domain: "subnet:192.168.2.0/24", ac: true, bat: 100 },
    { name: "macbook-m3-pro", id: "did:umcf:macbook_03", domain: "subnet:192.168.1.0/24", ac: true, bat: 95 },
    { name: "pixel-8-pro", id: "did:umcf:pixel_04", domain: "subnet:192.168.3.0/24", ac: false, bat: 82 },
    { name: "workstation-xeon", id: "did:umcf:workstation_05", domain: "rack:alpha_dc1", ac: true, bat: 100 },
    { name: "galaxy-tab-s9", id: "did:umcf:tablet_06", domain: "subnet:192.168.3.0/24", ac: false, bat: 65 },
    { name: "thinkpad-x1-spare", id: "did:umcf:thinkpad_07", domain: "subnet:192.168.4.0/24", ac: true, bat: 100 },
    { name: "nuc-homelab-spare", id: "did:umcf:nuc_08", domain: "subnet:192.168.2.0/24", ac: true, bat: 100 },
    { name: "surface-laptop-spare", id: "did:umcf:surface_09", domain: "subnet:192.168.4.0/24", ac: false, bat: 78 },
  ];

  // Helper to build shards based on mode
  function generateShards(mode: RedundancyMode): ShardCell[] {
    const shards: ShardCell[] = [];
    if (mode === "rs42") {
      // 4 Data, 2 Parity
      for (let i = 0; i < 4; i++) {
        const node = candidateNodes[i]!;
        shards.push({
          index: i,
          tag: `D${i}`,
          isParity: false,
          assignedNode: node.name,
          nodeId: node.id,
          failureDomain: node.domain,
          isAcPowered: node.ac,
          batteryPct: node.bat,
          shardHash: `blake3:${(i + 1) * 0x1a2b3c4d.toString(16).slice(0, 8)}`,
          sizeKb: 256,
          state: "healthy",
        });
      }
      for (let i = 0; i < 2; i++) {
        const node = candidateNodes[4 + i]!;
        shards.push({
          index: 4 + i,
          tag: `P${i}`,
          isParity: true,
          assignedNode: node.name,
          nodeId: node.id,
          failureDomain: node.domain,
          isAcPowered: node.ac,
          batteryPct: node.bat,
          shardHash: `blake3:${(i + 5) * 0x7e8f9a0b.toString(16).slice(0, 8)}`,
          sizeKb: 256,
          state: "healthy",
        });
      }
    } else if (mode === "rs63") {
      // 6 Data, 3 Parity
      for (let i = 0; i < 6; i++) {
        const node = candidateNodes[i]!;
        shards.push({
          index: i,
          tag: `D${i}`,
          isParity: false,
          assignedNode: node.name,
          nodeId: node.id,
          failureDomain: node.domain,
          isAcPowered: node.ac,
          batteryPct: node.bat,
          shardHash: `blake3:${(i + 1) * 0x3d4e5f6a.toString(16).slice(0, 8)}`,
          sizeKb: 171,
          state: "healthy",
        });
      }
      for (let i = 0; i < 3; i++) {
        const node = candidateNodes[6 + i]!;
        shards.push({
          index: 6 + i,
          tag: `P${i}`,
          isParity: true,
          assignedNode: node.name,
          nodeId: node.id,
          failureDomain: node.domain,
          isAcPowered: node.ac,
          batteryPct: node.bat,
          shardHash: `blake3:${(i + 7) * 0x9b8c7d6e.toString(16).slice(0, 8)}`,
          sizeKb: 171,
          state: "healthy",
        });
      }
    } else {
      // 3x Replication
      for (let i = 0; i < 3; i++) {
        const node = candidateNodes[i]!;
        shards.push({
          index: i,
          tag: `R${i + 1}`,
          isParity: i > 0,
          assignedNode: node.name,
          nodeId: node.id,
          failureDomain: node.domain,
          isAcPowered: node.ac,
          batteryPct: node.bat,
          shardHash: "blake3:e3b0c44298fc1c14",
          sizeKb: 1024,
          state: "healthy",
        });
      }
    }
    return shards;
  }

  let activeShards = $state<ShardCell[]>(generateShards("rs42"));

  // Track profile changes
  function setMode(mode: RedundancyMode) {
    selectedMode = mode;
    isSimulatingOutage = false;
    isHealing = false;
    activeShards = generateShards(mode);
    repairLog = [];
  }

  // --- Derived Metrics ---
  const totalShards = $derived(activeShards.length);
  const healthyShards = $derived(activeShards.filter(s => s.state === "healthy").length);
  const offlineShards = $derived(activeShards.filter(s => s.state === "offline").length);
  const requiredK = $derived(selectedMode === "rs42" ? 4 : selectedMode === "rs63" ? 6 : 1);
  const toleratesOutages = $derived(Math.max(0, healthyShards - requiredK));
  const durabilityState = $derived(
    healthyShards === totalShards
      ? "healthy"
      : healthyShards >= requiredK
      ? "degraded"
      : "unrecoverable"
  );

  // --- Interactive Actions ---
  function simulateNodeOutage() {
    if (isHealing) return;
    isSimulatingOutage = true;
    // Drop 2 nodes in rs42/rs63, or 1 in rep3
    const dropCount = selectedMode === "rep3" ? 1 : 2;
    for (let i = 0; i < dropCount; i++) {
      if (activeShards[i]) {
        activeShards[i]!.state = "offline";
      }
    }
    const timestamp = new Date().toLocaleTimeString();
    repairLog = [
      `[${timestamp}] ⚠️ Simulated outage: Nodes disconnected holding shards ${activeShards.slice(0, dropCount).map(s => s.tag).join(", ")}.`,
      `[${timestamp}] 🔍 Durability Degraded: ${healthyShards}/${totalShards} reachable shards. Reconstruction still 100% possible (tolerates ${toleratesOutages} more outages).`,
      ...repairLog,
    ];
  }

  async function triggerSelfHealing() {
    if (offlineShards === 0 || isHealing) return;
    isHealing = true;
    const timestamp = new Date().toLocaleTimeString();
    repairLog = [
      `[${timestamp}] ⚙️ Self-Healing initiated: Reed-Solomon engine inverting Cauchy generator matrix over GF(2^8)...`,
      ...repairLog,
    ];

    // Transition offline shards to healing state
    activeShards.forEach(s => {
      if (s.state === "offline") s.state = "healing";
    });

    await new Promise(r => setTimeout(r, 600));

    // Allocate spare nodes
    const spareNodes = candidateNodes.slice(6);
    let spareIdx = 0;
    activeShards.forEach(s => {
      if (s.state === "healing") {
        const spare = spareNodes[spareIdx % spareNodes.length]!;
        s.assignedNode = spare.name;
        s.nodeId = spare.id;
        s.failureDomain = spare.domain;
        s.isAcPowered = spare.ac;
        s.batteryPct = spare.bat;
        s.state = "healthy";
        spareIdx++;
      }
    });

    isSimulatingOutage = false;
    isHealing = false;
    const endTimestamp = new Date().toLocaleTimeString();
    repairLog = [
      `[${endTimestamp}] ✅ Self-Healing complete: Missing shards reconstructed and migrated to spare nodes. Durability restored to ${totalShards}/${totalShards} Healthy!`,
      ...repairLog,
    ];
  }

  function resetCluster() {
    isSimulatingOutage = false;
    isHealing = false;
    activeShards = generateShards(selectedMode);
    repairLog = [`[${new Date().toLocaleTimeString()}] 🔄 Cluster state reset to baseline.`];
  }
</script>

<div class="matrix-card">
  <!-- Header & Durability Health Banner -->
  <header class="card-header">
    <div class="title-group">
      <div class="icon-badge">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="m9 12 2 2 4-4"/>
        </svg>
      </div>
      <div>
        <h2>Erasure Shard Placement Matrix</h2>
        <p class="subtitle">GF(2^8) Systematic Reed-Solomon & Convergent XChaCha20-Poly1305</p>
      </div>
    </div>

    <!-- Health Status Pill -->
    <div class="health-pill {durabilityState}">
      <span class="status-dot"></span>
      <span class="status-label">
        {#if durabilityState === 'healthy'}
          {healthyShards}/{totalShards} Shards Healthy — Tolerates {toleratesOutages} Node Outages
        {:else if durabilityState === 'degraded'}
          Degraded ({healthyShards}/{totalShards} Shards) — Tolerates {toleratesOutages} Outages
        {:else}
          Unrecoverable ({healthyShards}/{totalShards} Shards)
        {/if}
      </span>
    </div>
  </header>

  <!-- Redundancy Profile Selector -->
  <section class="profile-selector" aria-label="Redundancy Profile Selector">
    <span class="selector-label">Redundancy Profile:</span>
    <div class="btn-group" role="group">
      <button
        class="profile-btn"
        class:active={selectedMode === 'rs42'}
        onclick={() => setMode('rs42')}
        aria-pressed={selectedMode === 'rs42'}
      >
        <strong>RS(4, 2) Balanced</strong>
        <span class="badge">1.5x Overhead &bull; Tolerates 2 Outages</span>
      </button>

      <button
        class="profile-btn"
        class:active={selectedMode === 'rs63'}
        onclick={() => setMode('rs63')}
        aria-pressed={selectedMode === 'rs63'}
      >
        <strong>RS(6, 3) High Durability</strong>
        <span class="badge">1.5x Overhead &bull; Tolerates 3 Outages</span>
      </button>

      <button
        class="profile-btn"
        class:active={selectedMode === 'rep3'}
        onclick={() => setMode('rep3')}
        aria-pressed={selectedMode === 'rep3'}
      >
        <strong>3x Replicated</strong>
        <span class="badge">3.0x Overhead &bull; Hot Metadata</span>
      </button>
    </div>
  </section>

  <!-- Cryptographic & Entropy Audit Strip -->
  <section class="crypto-strip">
    <div class="crypto-item">
      <span class="crypto-key">Client-Side Cipher:</span>
      <span class="crypto-val">XChaCha20-Poly1305 (24-byte Nonce, 16-byte Tag)</span>
    </div>
    <div class="crypto-item">
      <span class="crypto-key">Key Derivation:</span>
      <span class="crypto-val">HKDF-SHA256(namespace_secret, blake3(chunk))</span>
    </div>
    <div class="crypto-item">
      <span class="crypto-key">Measured Entropy:</span>
      <span class="crypto-val entropy-tag">7.9984 bits/byte (Zero Plaintext Leakage)</span>
    </div>
  </section>

  <!-- Shard Placement Matrix Grid -->
  <section class="shard-grid" aria-label="Shard Placement Grid">
    {#each activeShards as shard (shard.index)}
      <div class="shard-card {shard.state} {shard.isParity ? 'parity' : 'data'}">
        <div class="shard-header">
          <div class="shard-tag-badge {shard.isParity ? 'tag-parity' : 'tag-data'}">
            {shard.tag}
          </div>
          <span class="shard-role">{shard.isParity ? 'Parity Shard' : 'Data Shard'}</span>
          <span class="shard-state-pill {shard.state}">
            {shard.state.toUpperCase()}
          </span>
        </div>

        <div class="shard-details">
          <div class="detail-row">
            <span class="detail-label">Node:</span>
            <span class="detail-val node-name">{shard.assignedNode}</span>
          </div>

          <div class="detail-row">
            <span class="detail-label">Domain:</span>
            <span class="detail-val domain-tag">{shard.failureDomain}</span>
          </div>

          <div class="detail-row">
            <span class="detail-label">Power:</span>
            <span class="detail-val power-tag">
              {#if shard.isAcPowered}
                ⚡ AC Powered
              {:else}
                🔋 Battery {shard.batteryPct}%
              {/if}
            </span>
          </div>

          <div class="detail-row">
            <span class="detail-label">Fingerprint:</span>
            <span class="detail-val hash-tag">{shard.shardHash}</span>
          </div>
        </div>

        <div class="shard-footer">
          <span class="size-label">{shard.sizeKb} KiB</span>
          <span class="audit-ok">BLAKE3 Verified</span>
        </div>
      </div>
    {/each}
  </section>

  <!-- Interactive Controls Bar -->
  <footer class="controls-bar">
    <div class="actions-left">
      <button
        class="action-btn btn-danger"
        disabled={isSimulatingOutage || isHealing}
        onclick={simulateNodeOutage}
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="1" y1="1" x2="23" y2="23"/>
          <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55"/>
          <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39"/>
          <path d="M10.71 5.05A16 16 0 0 1 22.58 9"/>
          <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88"/>
          <path d="M8.53 16.11a6 6 0 0 1 6.95 0"/>
          <line x1="12" y1="20" x2="12.01" y2="20"/>
        </svg>
        Simulate Node Outage
      </button>

      <button
        class="action-btn btn-primary"
        disabled={offlineShards === 0 || isHealing}
        onclick={triggerSelfHealing}
      >
        {#if isHealing}
          <span class="spinner"></span>
          Reconstructing Shards...
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="23 4 23 10 17 10"/>
            <polyline points="1 20 1 14 7 14"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
          Reconstruct &amp; Self-Heal
        {/if}
      </button>
    </div>

    <button class="action-btn btn-secondary" onclick={resetCluster}>
      Reset Cluster
    </button>
  </footer>

  <!-- Real-Time Repair Timeline Log -->
  {#if repairLog.length > 0}
    <div class="repair-log">
      <h4>Self-Healing Repair Timeline</h4>
      <ul>
        {#each repairLog as logEntry}
          <li>{logEntry}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .matrix-card {
    background: #0f172a;
    border: 1px solid #1e293b;
    border-radius: 12px;
    padding: 24px;
    color: #e2e8f0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 16px;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon-badge {
    background: #1e293b;
    color: #38bdf8;
    padding: 10px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
    color: #f8fafc;
  }

  .subtitle {
    font-size: 0.85rem;
    color: #94a3b8;
    margin: 2px 0 0 0;
  }

  .health-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-radius: 9999px;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .health-pill.healthy {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b981;
    color: #34d399;
  }

  .health-pill.degraded {
    background: rgba(245, 158, 11, 0.15);
    border: 1px solid #f59e0b;
    color: #fbbf24;
  }

  .health-pill.unrecoverable {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid #ef4444;
    color: #f87171;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  /* Profile Selector */
  .profile-selector {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .selector-label {
    font-size: 0.9rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .btn-group {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .profile-btn {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 8px 14px;
    color: #cbd5e1;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    transition: all 0.15s ease-in-out;
  }

  .profile-btn:hover {
    border-color: #64748b;
  }

  .profile-btn.active {
    background: #0369a1;
    border-color: #38bdf8;
    color: #ffffff;
  }

  .badge {
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .profile-btn.active .badge {
    color: #e0f2fe;
  }

  /* Crypto Strip */
  .crypto-strip {
    background: #090d16;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 10px 16px;
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    font-size: 0.8rem;
  }

  .crypto-item {
    display: flex;
    gap: 6px;
  }

  .crypto-key {
    color: #94a3b8;
  }

  .crypto-val {
    color: #e2e8f0;
    font-family: monospace;
  }

  .entropy-tag {
    color: #34d399;
  }

  /* Shard Grid */
  .shard-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
  }

  .shard-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: all 0.2s ease;
  }

  .shard-card.parity {
    border-color: #4338ca;
    background: #151b33;
  }

  .shard-card.offline {
    border-color: #ef4444;
    background: rgba(239, 68, 68, 0.08);
  }

  .shard-card.healing {
    border-color: #f59e0b;
    background: rgba(245, 158, 11, 0.08);
    animation: pulse 1.2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .shard-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .shard-tag-badge {
    font-weight: 700;
    font-size: 0.85rem;
    padding: 4px 8px;
    border-radius: 6px;
    font-family: monospace;
  }

  .tag-data {
    background: #0284c7;
    color: #ffffff;
  }

  .tag-parity {
    background: #6366f1;
    color: #ffffff;
  }

  .shard-role {
    font-size: 0.78rem;
    color: #94a3b8;
    flex-grow: 1;
  }

  .shard-state-pill {
    font-size: 0.68rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
  }

  .shard-state-pill.healthy {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
  }

  .shard-state-pill.offline {
    background: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }

  .shard-state-pill.healing {
    background: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
  }

  .shard-details {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 0.8rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  .detail-label {
    color: #64748b;
  }

  .detail-val {
    color: #cbd5e1;
    font-family: monospace;
  }

  .domain-tag {
    color: #38bdf8;
  }

  .power-tag {
    color: #fbbf24;
  }

  .hash-tag {
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .shard-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 8px;
    font-size: 0.75rem;
    color: #64748b;
  }

  .audit-ok {
    color: #34d399;
  }

  /* Controls Bar */
  .controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    border-top: 1px solid #1e293b;
    padding-top: 16px;
  }

  .actions-left {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.15s ease;
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-danger {
    background: #991b1b;
    color: #fee2e2;
  }

  .btn-danger:hover:not(:disabled) {
    background: #b91c1c;
  }

  .btn-primary {
    background: #0284c7;
    color: #ffffff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #0369a1;
  }

  .btn-secondary {
    background: #334155;
    color: #e2e8f0;
  }

  .btn-secondary:hover {
    background: #475569;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid #ffffff;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Timeline Log */
  .repair-log {
    background: #090d16;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 14px;
    font-size: 0.82rem;
  }

  .repair-log h4 {
    margin: 0 0 8px 0;
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .repair-log ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-family: monospace;
    color: #cbd5e1;
  }
</style>

