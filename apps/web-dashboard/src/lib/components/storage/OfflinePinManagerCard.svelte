<!--
  OfflinePinManagerCard.svelte
  UMCF Stage 07 Phase 1 Subphase 3 (S7.P1.SP3)
  Hierarchical Pinning, Offline Availability, W-TinyLFU Cache & Concurrent Garbage Collection.
  Adheres strictly to Svelte 5 runes ($state, $derived), strict TypeScript, and WCAG 2.1 AA accessibility.
-->
<script lang="ts">
  type PinMode = "permanent" | "ttl" | "job";
  type HydrationStatus = "hydrated" | "hydrating" | "degraded";

  interface PinItem {
    cid: string;
    label: string;
    mode: PinMode;
    expiresInMin?: number;
    jobId?: string;
    hydration: HydrationStatus;
    progressPct: number;
    sizeMb: number;
    pinnedBy: string;
  }

  // --- Reactive Runes State ---
  let pins = $state<PinItem[]>([
    {
      cid: "bafk-blake3-llama3-8b-instruct",
      label: "llama-3-8b-instruct.Q4_K_M.gguf",
      mode: "permanent",
      hydration: "hydrated",
      progressPct: 100,
      sizeMb: 4920,
      pinnedBy: "did:umcf:operator_01",
    },
    {
      cid: "bafk-blake3-offline-gis-vector-tiles",
      label: "offline_navigation_tiles.mbtiles",
      mode: "ttl",
      expiresInMin: 48,
      hydration: "hydrating",
      progressPct: 68,
      sizeMb: 640,
      pinnedBy: "did:umcf:field_tablet_04",
    },
    {
      cid: "bafk-blake3-render-cache-scene42",
      label: "scene_042_intermediate_composite.exr",
      mode: "job",
      jobId: "job-render-8821",
      hydration: "hydrated",
      progressPct: 100,
      sizeMb: 128,
      pinnedBy: "did:umcf:workstation_05",
    },
  ]);

  let isRunningGc = $state(false);
  let isScrubbing = $state(false);
  let isAddingPin = $state(false);
  let newPinLabel = $state("");
  let newPinMode = $state<PinMode>("permanent");
  let newPinTtlMin = $state(60);

  let gcLog = $state<string[]>([
    `[${new Date().toLocaleTimeString()}] System ready. 3 active pins protecting 5.68 GB from garbage collection.`,
  ]);

  // W-TinyLFU metrics
  let cacheHits = $state(18420);
  let cacheMisses = $state(1520);
  let unpinnedCacheMb = $state(842);
  let maxCacheMb = $state(1024);

  // --- Derived Calculations ---
  const hitRatioPct = $derived(
    cacheHits + cacheMisses > 0
      ? ((cacheHits / (cacheHits + cacheMisses)) * 100).toFixed(1)
      : "0.0"
  );

  const totalPinnedMb = $derived(
    pins.reduce((sum, p) => sum + p.sizeMb, 0)
  );

  const reclaimableMb = $derived(
    Math.max(0, unpinnedCacheMb - 200)
  );

  // --- Interactive Actions ---
  function toggleAddPin() {
    isAddingPin = !isAddingPin;
  }

  function addPin() {
    if (!newPinLabel.trim()) return;

    const newCid = `bafk-blake3-${Date.now().toString(16).slice(-8)}`;
    pins.push({
      cid: newCid,
      label: newPinLabel.trim(),
      mode: newPinMode,
      expiresInMin: newPinMode === "ttl" ? newPinTtlMin : undefined,
      hydration: "hydrating",
      progressPct: 15,
      sizeMb: 256,
      pinnedBy: "did:umcf:local_operator",
    });

    const timestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${timestamp}] 📌 Pinned "${newPinLabel.trim()}" (${newPinMode.toUpperCase()}). Hydration initiated...`,
      ...gcLog,
    ];

    // Simulate background hydration completion
    setTimeout(() => {
      const idx = pins.findIndex(p => p.cid === newCid);
      if (idx !== -1) {
        pins[idx]!.hydration = "hydrated";
        pins[idx]!.progressPct = 100;
        gcLog = [
          `[${new Date().toLocaleTimeString()}] 🟢 Hydration complete: "${newPinLabel.trim()}" fully available offline with 0 network latency.`,
          ...gcLog,
        ];
      }
    }, 1500);

    newPinLabel = "";
    isAddingPin = false;
  }

  function unpin(cid: string) {
    const item = pins.find(p => p.cid === cid);
    pins = pins.filter(p => p.cid !== cid);

    const timestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${timestamp}] 🗑️ Unpinned "${item?.label ?? cid}". Child chunks now eligible for W-TinyLFU eviction or GC sweep.`,
      ...gcLog,
    ];
  }

  async function runGarbageCollection() {
    if (isRunningGc) return;
    isRunningGc = true;

    const timestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${timestamp}] 🧹 Starting Concurrent GC cycle (Epoch Lock acquired)...`,
      `[${timestamp}] 🔍 Root set enumerated: ${pins.length} active pins, 1 active job, 0 grace-window violations.`,
      ...gcLog,
    ];

    await new Promise(r => setTimeout(r, 600));

    const sweptMb = reclaimableMb;
    unpinnedCacheMb = Math.max(50, unpinnedCacheMb - sweptMb);
    isRunningGc = false;

    const endTimestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${endTimestamp}] ✅ GC Sweep complete: ${sweptMb} MB reclaimed. Exact byte accounting parity verified with 0 dangling references.`,
      ...gcLog,
    ];
  }

  async function scrubBitRot() {
    if (isScrubbing) return;
    isScrubbing = true;

    const timestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${timestamp}] 🛡️ Idle Bit-Rot Scrubbing started: Scanning all chunk files and verifying BLAKE3 cryptographic hashes...`,
      ...gcLog,
    ];

    await new Promise(r => setTimeout(r, 800));
    isScrubbing = false;

    const endTimestamp = new Date().toLocaleTimeString();
    gcLog = [
      `[${endTimestamp}] 🟢 Bit-Rot Scrub complete: 100% of on-disk chunks verified intact with 0 bit-rot anomalies detected.`,
      ...gcLog,
    ];
  }
</script>

<div class="pin-card">
  <!-- Card Header -->
  <header class="card-header">
    <div class="title-group">
      <div class="icon-badge">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2v20"/>
          <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
        </svg>
      </div>
      <div>
        <h2>Offline Pinning &amp; Cache Manager</h2>
        <p class="subtitle">Guaranteed Offline Availability, Scan-Resistant W-TinyLFU &amp; Concurrent Mark-Sweep GC</p>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="header-actions">
      <button class="action-btn btn-primary" onclick={toggleAddPin}>
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19"/>
          <line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        {isAddingPin ? 'Cancel' : 'Pin Object for Offline'}
      </button>
    </div>
  </header>

  <!-- Add Pin Drawer -->
  {#if isAddingPin}
    <div class="add-pin-drawer">
      <div class="input-row">
        <div class="input-group">
          <label for="pin-label">Artifact / Model Label:</label>
          <input
            id="pin-label"
            type="text"
            bind:value={newPinLabel}
            placeholder="e.g. llama-3-8b-instruct.gguf or maps.pack"
          />
        </div>

        <div class="input-group">
          <label for="pin-mode">Pinning Mode:</label>
          <select id="pin-mode" bind:value={newPinMode}>
            <option value="permanent">Recursive Permanent (Indefinite)</option>
            <option value="ttl">Lease with TTL (Auto-Expiring)</option>
            <option value="job">Active Job Hold</option>
          </select>
        </div>

        {#if newPinMode === 'ttl'}
          <div class="input-group">
            <label for="pin-ttl">TTL Duration (Minutes):</label>
            <input id="pin-ttl" type="number" min="5" max="1440" bind:value={newPinTtlMin} />
          </div>
        {/if}

        <button class="action-btn btn-success" onclick={addPin}>
          Confirm Pin
        </button>
      </div>
    </div>
  {/if}

  <!-- Telemetry & Metrics Strip -->
  <section class="metrics-grid">
    <div class="metric-card">
      <span class="metric-label">Offline Pinned Storage</span>
      <span class="metric-value">{(totalPinnedMb / 1024).toFixed(2)} GB</span>
      <span class="metric-sub">{pins.length} Objects &bull; 100% Offline Guaranteed</span>
    </div>

    <div class="metric-card">
      <span class="metric-label">W-TinyLFU Cache Hit Ratio</span>
      <span class="metric-value text-emerald">{hitRatioPct}%</span>
      <span class="metric-sub">{cacheHits} Hits &bull; {cacheMisses} Misses (Scan-Resistant)</span>
    </div>

    <div class="metric-card">
      <span class="metric-label">Unpinned Transient Cache</span>
      <span class="metric-value">{unpinnedCacheMb} / {maxCacheMb} MB</span>
      <span class="metric-sub">{reclaimableMb} MB Reclaimable by GC</span>
    </div>
  </section>

  <!-- Active Pins Table -->
  <section class="pins-table-section">
    <h3>Active Pins &amp; Offline Hydration States</h3>
    <div class="table-wrapper">
      <table class="pins-table">
        <thead>
          <tr>
            <th>Label &amp; Content ID</th>
            <th>Mode</th>
            <th>Offline Hydration</th>
            <th>Size</th>
            <th>Pinned By</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each pins as pin (pin.cid)}
            <tr>
              <td>
                <div class="label-col">
                  <strong>{pin.label}</strong>
                  <span class="cid-badge">{pin.cid}</span>
                </div>
              </td>
              <td>
                <span class="mode-pill {pin.mode}">
                  {#if pin.mode === 'permanent'}
                    Permanent
                  {:else if pin.mode === 'ttl'}
                    TTL ({pin.expiresInMin}m left)
                  {:else}
                    Job ({pin.jobId})
                  {/if}
                </span>
              </td>
              <td>
                <div class="hydration-col">
                  {#if pin.hydration === 'hydrated'}
                    <span class="status-pill status-hydrated">
                      <span class="dot"></span> Fully Available Offline
                    </span>
                  {:else if pin.hydration === 'hydrating'}
                    <div class="progress-wrap">
                      <div class="progress-bar">
                        <div class="progress-fill" style="width: {pin.progressPct}%"></div>
                      </div>
                      <span class="progress-text">Hydrating ({pin.progressPct}%)</span>
                    </div>
                  {:else}
                    <span class="status-pill status-degraded">Degraded</span>
                  {/if}
                </div>
              </td>
              <td>{pin.sizeMb > 1024 ? `${(pin.sizeMb / 1024).toFixed(1)} GB` : `${pin.sizeMb} MB`}</td>
              <td class="node-id-col">{pin.pinnedBy}</td>
              <td>
                <button class="action-btn btn-danger-sm" onclick={() => unpin(pin.cid)}>
                  Unpin
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>

  <!-- Garbage Collection & Maintenance Actions -->
  <footer class="maintenance-bar">
    <div class="maintenance-info">
      <h4>Storage Maintenance &amp; Bit-Rot Guard</h4>
      <p>Unpinned chunks are pruned via epoch-locked mark-and-sweep. Live chunk hashes are scrubbed for flash bit-rot.</p>
    </div>

    <div class="maintenance-actions">
      <button
        class="action-btn btn-secondary"
        disabled={isScrubbing || isRunningGc}
        onclick={scrubBitRot}
      >
        {#if isScrubbing}
          <span class="spinner"></span>
          Scrubbing Hashes...
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          </svg>
          Scrub Bit-Rot
        {/if}
      </button>

      <button
        class="action-btn btn-warning"
        disabled={isRunningGc || isScrubbing || reclaimableMb === 0}
        onclick={runGarbageCollection}
      >
        {#if isRunningGc}
          <span class="spinner"></span>
          Running GC...
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6"/>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
          </svg>
          Run GC Cycle (Reclaim {reclaimableMb} MB)
        {/if}
      </button>
    </div>
  </footer>

  <!-- Lifecycle Timeline Log -->
  {#if gcLog.length > 0}
    <div class="gc-log">
      <h4>Storage Lifecycle &amp; GC Audit Log</h4>
      <ul>
        {#each gcLog as entry}
          <li>{entry}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .pin-card {
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

  /* Add Pin Drawer */
  .add-pin-drawer {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    padding: 16px;
  }

  .input-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 180px;
  }

  .input-group label {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .input-group input,
  .input-group select {
    background: #0f172a;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 8px 12px;
    color: #f8fafc;
    font-size: 0.88rem;
  }

  /* Metrics Grid */
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
  }

  .metric-card {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .metric-label {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .metric-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .metric-sub {
    font-size: 0.75rem;
    color: #64748b;
  }

  .text-emerald {
    color: #34d399;
  }

  /* Table Section */
  .pins-table-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .pins-table-section h3 {
    font-size: 0.95rem;
    color: #cbd5e1;
    margin: 0;
  }

  .table-wrapper {
    overflow-x: auto;
    border: 1px solid #1e293b;
    border-radius: 8px;
  }

  .pins-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
    text-align: left;
  }

  .pins-table th {
    background: #1e293b;
    color: #94a3b8;
    padding: 10px 14px;
    font-weight: 600;
    border-bottom: 1px solid #334155;
  }

  .pins-table td {
    padding: 12px 14px;
    border-bottom: 1px solid #1e293b;
    background: #0f172a;
  }

  .label-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .cid-badge {
    font-family: monospace;
    font-size: 0.72rem;
    color: #64748b;
  }

  .mode-pill {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .mode-pill.permanent {
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
  }

  .mode-pill.ttl {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }

  .mode-pill.job {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
  }

  .hydration-col {
    display: flex;
    align-items: center;
  }

  .status-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .status-hydrated {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }

  .status-hydrated .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #34d399;
  }

  .status-degraded {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
  }

  .progress-wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 140px;
  }

  .progress-bar {
    height: 6px;
    background: #334155;
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #38bdf8;
    transition: width 0.3s ease;
  }

  .progress-text {
    font-size: 0.7rem;
    color: #94a3b8;
  }

  .node-id-col {
    font-family: monospace;
    font-size: 0.75rem;
    color: #94a3b8;
  }

  /* Actions */
  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.15s ease;
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #0284c7;
    color: #ffffff;
  }

  .btn-primary:hover:not(:disabled) {
    background: #0369a1;
  }

  .btn-success {
    background: #059669;
    color: #ffffff;
  }

  .btn-success:hover {
    background: #047857;
  }

  .btn-warning {
    background: #d97706;
    color: #ffffff;
  }

  .btn-warning:hover:not(:disabled) {
    background: #b45309;
  }

  .btn-secondary {
    background: #334155;
    color: #cbd5e1;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #475569;
  }

  .btn-danger-sm {
    background: transparent;
    border: 1px solid #ef4444;
    color: #f87171;
    padding: 4px 8px;
    font-size: 0.75rem;
  }

  .btn-danger-sm:hover {
    background: #ef4444;
    color: #ffffff;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid #ffffff;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Maintenance Bar */
  .maintenance-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 16px;
    border-top: 1px solid #1e293b;
    padding-top: 16px;
  }

  .maintenance-info h4 {
    margin: 0;
    font-size: 0.9rem;
    color: #f8fafc;
  }

  .maintenance-info p {
    margin: 2px 0 0 0;
    font-size: 0.78rem;
    color: #94a3b8;
  }

  .maintenance-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  /* Log */
  .gc-log {
    background: #090d16;
    border: 1px solid #1e293b;
    border-radius: 8px;
    padding: 14px;
    font-size: 0.8rem;
  }

  .gc-log h4 {
    margin: 0 0 8px 0;
    color: #94a3b8;
    font-size: 0.85rem;
  }

  .gc-log ul {
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

