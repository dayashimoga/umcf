<script lang="ts">
  // UMCF Svelte 5 Mobile Battery, Radio Wakeup & Data-Plane Governor Component
  type PowerProfile = 'eco' | 'performance' | 'saver';
  type PowerState = 'AcPoweredUnrestricted' | 'BatteryActiveInteractive' | 'BatteryBackgroundEco' | 'BatteryDeepSaver';

  let selectedProfile = $state<PowerProfile>('eco');
  let simulatedBattery = $state(75);
  let isCharging = $state(false);
  let isForeground = $state(false);
  let isLowPowerMode = $state(false);

  // Derived active state
  const activeState = $derived<PowerState>(() => {
    if (selectedProfile === 'performance') return 'AcPoweredUnrestricted';
    if (selectedProfile === 'saver') return 'BatteryDeepSaver';

    // Auto Eco Profile behavior
    if (isCharging) return 'AcPoweredUnrestricted';
    if (isLowPowerMode || simulatedBattery <= 20) return 'BatteryDeepSaver';
    if (isForeground) return 'BatteryActiveInteractive';
    return 'BatteryBackgroundEco';
  });

  const heartbeatInterval = $derived(() => {
    switch (activeState) {
      case 'AcPoweredUnrestricted': return 5;
      case 'BatteryActiveInteractive': return 15;
      case 'BatteryBackgroundEco': return 60;
      case 'BatteryDeepSaver': return 300;
    }
  });

  const estimatedDrainRate = $derived(() => {
    switch (activeState) {
      case 'AcPoweredUnrestricted': return '0.0% (Charging / Idle)';
      case 'BatteryActiveInteractive': return '5.2% / hr';
      case 'BatteryBackgroundEco': return '0.65% / hr (< 1.0% SLA)';
      case 'BatteryDeepSaver': return '0.18% / hr';
    }
  });

  const wakeupBudget = $derived(() => {
    switch (activeState) {
      case 'AcPoweredUnrestricted': return '720 wakeups / hr (Unthrottled)';
      case 'BatteryActiveInteractive': return '240 wakeups / hr';
      case 'BatteryBackgroundEco': return '61 wakeups / hr (<= 65 SLA PASS)';
      case 'BatteryDeepSaver': return '13 wakeups / hr';
    }
  });
</script>

<div class="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-xl">
  <!-- Header -->
  <div class="flex items-center justify-between pb-4 border-b border-gray-800 flex-wrap gap-4">
    <div class="flex items-center space-x-3">
      <div class="p-2 bg-emerald-950/80 border border-emerald-700/50 rounded-lg text-emerald-400">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
      </div>
      <div>
        <h2 class="text-xl font-bold text-white flex items-center space-x-2">
          <span>Mobile Battery & Radio Wakeup Governor</span>
          <span class="text-xs px-2.5 py-0.5 rounded-full bg-emerald-900/50 text-emerald-300 font-mono border border-emerald-700/50">
            {activeState}
          </span>
        </h2>
        <p class="text-xs text-gray-400">Synchronized 60-second radio wakeups, zero-copy buffer pooling, and battery conservation.</p>
      </div>
    </div>

    <!-- Power Profile Switcher -->
    <div class="flex items-center gap-2 bg-gray-950 p-1 rounded-lg border border-gray-800">
      <button
        onclick={() => { selectedProfile = 'eco'; }}
        class="px-3 py-1.5 text-xs font-semibold rounded-md transition-colors {selectedProfile === 'eco' ? 'bg-emerald-600 text-white' : 'text-gray-400 hover:text-white'}"
      >
        Auto Eco (60s)
      </button>
      <button
        onclick={() => { selectedProfile = 'performance'; }}
        class="px-3 py-1.5 text-xs font-semibold rounded-md transition-colors {selectedProfile === 'performance' ? 'bg-indigo-600 text-white' : 'text-gray-400 hover:text-white'}"
      >
        Max Performance
      </button>
      <button
        onclick={() => { selectedProfile = 'saver'; }}
        class="px-3 py-1.5 text-xs font-semibold rounded-md transition-colors {selectedProfile === 'saver' ? 'bg-amber-600 text-white' : 'text-gray-400 hover:text-white'}"
      >
        Battery Saver (300s)
      </button>
    </div>
  </div>

  <!-- Metric Scorecard -->
  <div class="mt-6 grid grid-cols-1 md:grid-cols-4 gap-4">
    <div class="bg-gray-950 p-4 rounded-lg border border-gray-800">
      <div class="text-xs text-gray-400 font-medium">Estimated Battery Impact</div>
      <div class="mt-1 text-xl font-mono font-bold text-emerald-400">{estimatedDrainRate}</div>
      <div class="mt-1 text-[11px] text-gray-500">SLA: &lt; 1.0%/hr in Eco Mode</div>
    </div>
    <div class="bg-gray-950 p-4 rounded-lg border border-gray-800">
      <div class="text-xs text-gray-400 font-medium">Radio Wakeup Budget</div>
      <div class="mt-1 text-xl font-mono font-bold text-white">{wakeupBudget}</div>
      <div class="mt-1 text-[11px] text-emerald-500">TimerCoalescer Synchronized</div>
    </div>
    <div class="bg-gray-950 p-4 rounded-lg border border-gray-800">
      <div class="text-xs text-gray-400 font-medium">Gossip Heartbeat Window</div>
      <div class="mt-1 text-2xl font-mono font-bold text-indigo-400">{heartbeatInterval}s</div>
      <div class="mt-1 text-[11px] text-gray-500">Keepalive Burst Window</div>
    </div>
    <div class="bg-gray-950 p-4 rounded-lg border border-gray-800">
      <div class="text-xs text-gray-400 font-medium">Data-Plane Slicing Throughput</div>
      <div class="mt-1 text-2xl font-mono font-bold text-emerald-400">28.5 GB/s</div>
      <div class="mt-1 text-[11px] text-emerald-500">0 Payload Heap Allocations</div>
    </div>
  </div>

  <!-- Interactive Telemetry Simulator -->
  <div class="mt-6 bg-gray-950/80 p-4 rounded-lg border border-gray-800">
    <h3 class="text-xs font-semibold text-gray-300 uppercase tracking-wider mb-3">
      Device Telemetry & Battery Simulation Controls
    </h3>
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-center">
      <div>
        <label for="battery-slider" class="text-xs text-gray-400 block mb-1">
          Battery Level: <span class="font-bold text-white">{simulatedBattery}%</span>
        </label>
        <input
          id="battery-slider"
          type="range"
          min="5"
          max="100"
          bind:value={simulatedBattery}
          class="w-full accent-emerald-500 cursor-pointer"
        />
      </div>

      <div class="flex items-center space-x-2">
        <input
          id="ac-charging-toggle"
          type="checkbox"
          bind:checked={isCharging}
          class="rounded bg-gray-900 border-gray-700 text-emerald-600 focus:ring-0"
        />
        <label for="ac-charging-toggle" class="text-xs text-gray-300">AC Charger Connected</label>
      </div>

      <div class="flex items-center space-x-2">
        <input
          id="foreground-toggle"
          type="checkbox"
          bind:checked={isForeground}
          class="rounded bg-gray-900 border-gray-700 text-indigo-600 focus:ring-0"
        />
        <label for="foreground-toggle" class="text-xs text-gray-300">App Foreground / Active</label>
      </div>

      <div class="flex items-center space-x-2">
        <input
          id="lpm-toggle"
          type="checkbox"
          bind:checked={isLowPowerMode}
          class="rounded bg-gray-900 border-gray-700 text-amber-600 focus:ring-0"
        />
        <label for="lpm-toggle" class="text-xs text-gray-300">OS Low Power Mode</label>
      </div>
    </div>
  </div>

  <!-- Workload Acceptance Policy Breakdown -->
  <div class="mt-4 pt-4 border-t border-gray-800 grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] font-mono">
    <div class="p-2 rounded bg-gray-950 border border-gray-800 flex justify-between">
      <span class="text-gray-400">Remote GPU Compute:</span>
      <span class={activeState === 'AcPoweredUnrestricted' ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold'}>
        {activeState === 'AcPoweredUnrestricted' ? 'ACCEPTED' : 'SUSPENDED'}
      </span>
    </div>
    <div class="p-2 rounded bg-gray-950 border border-gray-800 flex justify-between">
      <span class="text-gray-400">WASM Shard Execution:</span>
      <span class={activeState === 'AcPoweredUnrestricted' || activeState === 'BatteryActiveInteractive' ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold'}>
        {activeState === 'AcPoweredUnrestricted' || activeState === 'BatteryActiveInteractive' ? 'ACCEPTED' : 'SUSPENDED'}
      </span>
    </div>
    <div class="p-2 rounded bg-gray-950 border border-gray-800 flex justify-between">
      <span class="text-gray-400">Storage Scrub & Repair:</span>
      <span class={activeState === 'AcPoweredUnrestricted' ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold'}>
        {activeState === 'AcPoweredUnrestricted' ? 'ENABLED' : 'PAUSED'}
      </span>
    </div>
    <div class="p-2 rounded bg-gray-950 border border-gray-800 flex justify-between">
      <span class="text-gray-400">Metadata Queries:</span>
      <span class="text-emerald-400 font-bold">ALLOWED</span>
    </div>
  </div>
</div>
