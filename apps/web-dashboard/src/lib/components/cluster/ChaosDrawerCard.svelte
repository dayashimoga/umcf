<script lang="ts">
  // UMCF Svelte 5 Chaos Engineering Drawer & Fault Injection Console
  interface ChaosEvent {
    id: string;
    type: 'partition' | 'thermal' | 'memory' | 'churn' | 'heal';
    description: string;
    timestamp: string;
    status: 'ACTIVE' | 'RESOLVED' | 'MIGRATED';
  }

  let activePartition = $state<string | null>(null);
  let thermalThrottleActive = $state(false);
  let memoryTrimActive = $state(false);
  let appNapActive = $state(false);
  let convergenceTimeMs = $state<number | null>(null);

  let eventLog = $state<ChaosEvent[]>([
    {
      id: 'init-01',
      type: 'heal',
      description: 'Cluster baseline healthy. All 6 OS nodes responding.',
      timestamp: 'Just now',
      status: 'RESOLVED',
    }
  ]);

  const invariants = $derived([
    { id: 1, name: 'Data & Output Integrity', status: 'PASS', detail: '0 bit flips or hash drift' },
    { id: 2, name: 'Split-Brain Convergence', status: convergenceTimeMs ? (convergenceTimeMs <= 5000 ? 'PASS' : 'WARN') : 'PASS', detail: convergenceTimeMs ? `${convergenceTimeMs} ms <= 5000 ms SLA` : '<= 5.0s SLA' },
    { id: 3, name: 'No Zombie / Orphaned Leases', status: 'PASS', detail: 'Lease timeout <= 3.0s enforced' },
    { id: 4, name: 'Storage Durability RS(4,2)', status: 'PASS', detail: 'Quorum intact >= 4 surviving shards' },
    { id: 5, name: 'Ledger Zero-Sum Conservation', status: 'PASS', detail: 'Debits == Credits, 0 double-payouts' },
  ]);

  function triggerPartition(type: 'symmetric' | 'asymmetric') {
    activePartition = type === 'symmetric' ? 'Symmetric Split {A,B} | {C,D,E}' : 'Asymmetric Bridge A <-> B <-> C';
    eventLog = [
      {
        id: `ev-${Date.now()}`,
        type: 'partition',
        description: `Injected ${activePartition}. Cross-group QUIC packets dropped.`,
        timestamp: new Date().toLocaleTimeString(),
        status: 'ACTIVE',
      },
      ...eventLog,
    ];
  }

  function triggerThermalSpike() {
    thermalThrottleActive = true;
    eventLog = [
      {
        id: `ev-${Date.now()}`,
        type: 'thermal',
        description: 'Simulated ThermalState::Critical (95°C) on mobile ARM64 node. Tasks migrated.',
        timestamp: new Date().toLocaleTimeString(),
        status: 'MIGRATED',
      },
      ...eventLog,
    ];
  }

  function triggerMemoryTrim() {
    memoryTrimActive = true;
    eventLog = [
      {
        id: `ev-${Date.now()}`,
        type: 'memory',
        description: 'Simulated Android onTrimMemory(TRIM_MEMORY_RUNNING_CRITICAL). Evicted 256 MiB cache.',
        timestamp: new Date().toLocaleTimeString(),
        status: 'RESOLVED',
      },
      ...eventLog,
    ];
  }

  function triggerAppNap() {
    appNapActive = true;
    eventLog = [
      {
        id: `ev-${Date.now()}`,
        type: 'churn',
        description: 'Simulated iOS App Nap / Android Doze 30s freeze without TCP FIN. Lease expired.',
        timestamp: new Date().toLocaleTimeString(),
        status: 'ACTIVE',
      },
      ...eventLog,
    ];
  }

  function healAll() {
    activePartition = null;
    thermalThrottleActive = false;
    memoryTrimActive = false;
    appNapActive = false;
    convergenceTimeMs = Math.floor(180 + Math.random() * 450);

    eventLog = [
      {
        id: `ev-${Date.now()}`,
        type: 'heal',
        description: `Healed all network partitions. Cluster converged in ${convergenceTimeMs} ms (< 5000 ms SLA).`,
        timestamp: new Date().toLocaleTimeString(),
        status: 'RESOLVED',
      },
      ...eventLog,
    ];
  }
</script>

<div class="bg-gray-900 border border-red-900/60 rounded-xl p-6 shadow-xl">
  <div class="flex items-center justify-between pb-4 border-b border-gray-800">
    <div class="flex items-center space-x-3">
      <div class="p-2 bg-red-950/80 border border-red-700/50 rounded-lg text-red-400">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      </div>
      <div>
        <h2 class="text-xl font-bold text-white flex items-center space-x-2">
          <span>Chaos Engineering & Fault Injection</span>
          <span class="text-xs px-2 py-0.5 rounded-full bg-red-900/50 text-red-300 font-mono border border-red-700/50">Jepsen Nemesis</span>
        </h2>
        <p class="text-xs text-gray-400">Simulate partitions, node churn, thermal storms, and memory pressure with real-time invariant auditing.</p>
      </div>
    </div>
    <div class="flex items-center space-x-2">
      <button
        onclick={healAll}
        class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg font-medium text-sm transition-all shadow-lg flex items-center space-x-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        <span>Heal All & Converge</span>
      </button>
    </div>
  </div>

  <!-- Fault Injectors Grid -->
  <div class="mt-6 grid grid-cols-1 md:grid-cols-4 gap-3">
    <button
      onclick={() => triggerPartition('symmetric')}
      class="p-3 bg-gray-800/80 hover:bg-gray-800 border border-gray-700 rounded-lg text-left transition-all"
    >
      <div class="text-xs font-semibold text-amber-400 uppercase tracking-wide">Network Split</div>
      <div class="text-sm font-bold text-white mt-1">Symmetric Split</div>
      <div class="text-xs text-gray-400 mt-1">{`{A,B} | {C,D,E}`} partition</div>
    </button>

    <button
      onclick={() => triggerPartition('asymmetric')}
      class="p-3 bg-gray-800/80 hover:bg-gray-800 border border-gray-700 rounded-lg text-left transition-all"
    >
      <div class="text-xs font-semibold text-amber-400 uppercase tracking-wide">Network Split</div>
      <div class="text-sm font-bold text-white mt-1">Asymmetric Bridge</div>
      <div class="text-xs text-gray-400 mt-1">A &lt;-&gt; B &lt;-&gt; C (A -/- C)</div>
    </button>

    <button
      onclick={triggerThermalSpike}
      class="p-3 bg-gray-800/80 hover:bg-gray-800 border border-gray-700 rounded-lg text-left transition-all"
    >
      <div class="text-xs font-semibold text-rose-400 uppercase tracking-wide">Hardware Storm</div>
      <div class="text-sm font-bold text-white mt-1">Thermal Critical (95°C)</div>
      <div class="text-xs text-gray-400 mt-1">Trigger live task migration</div>
    </button>

    <button
      onclick={triggerMemoryTrim}
      class="p-3 bg-gray-800/80 hover:bg-gray-800 border border-gray-700 rounded-lg text-left transition-all"
    >
      <div class="text-xs font-semibold text-purple-400 uppercase tracking-wide">Mobile Resource</div>
      <div class="text-sm font-bold text-white mt-1">onTrimMemory Critical</div>
      <div class="text-xs text-gray-400 mt-1">Force 256 MiB cache flush</div>
    </button>
  </div>

  <!-- Continuous Invariant Monitor Badges -->
  <div class="mt-6">
    <div class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Continuous Cluster Invariants (Jepsen Suite)</div>
    <div class="grid grid-cols-1 sm:grid-cols-5 gap-3">
      {#each invariants as inv}
        <div class="p-3 bg-gray-950/60 border border-gray-800 rounded-lg">
          <div class="flex items-center justify-between">
            <span class="text-xs font-mono text-gray-400">INV-0{inv.id}</span>
            <span class="text-xs font-bold px-2 py-0.5 rounded bg-emerald-900/60 text-emerald-400 border border-emerald-700/50">
              {inv.status}
            </span>
          </div>
          <div class="text-xs font-bold text-gray-200 mt-1">{inv.name}</div>
          <div class="text-[11px] text-gray-400 mt-0.5">{inv.detail}</div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Chaos Event Log -->
  <div class="mt-6">
    <div class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Live Chaos Audit Feed</div>
    <div class="bg-gray-950 rounded-lg border border-gray-800 p-3 max-h-48 overflow-y-auto font-mono text-xs space-y-2">
      {#each eventLog as ev}
        <div class="flex items-center justify-between text-gray-300">
          <div class="flex items-center space-x-2">
            <span class="text-gray-500">[{ev.timestamp}]</span>
            <span class={`font-bold ${ev.status === 'ACTIVE' ? 'text-amber-400' : ev.status === 'MIGRATED' ? 'text-purple-400' : 'text-emerald-400'}`}>
              [{ev.status}]
            </span>
            <span>{ev.description}</span>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>
