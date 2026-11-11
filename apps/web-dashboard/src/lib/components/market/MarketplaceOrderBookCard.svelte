<script lang="ts">
  // Svelte 5 Runes for UMCF Marketplace & Double-Entry Credit Ledger

  interface AccountBalances {
    available_micro: number;
    escrow_locked_micro: number;
    provider_stake_micro: number;
    slashed_pool_micro: number;
  }

  interface AskOfferUI {
    offer_id: string;
    provider_node_id: string;
    cpu_cores: number;
    ram_mb: number;
    gpu_backend: string | null;
    vram_mb: number;
    npu_tops: number;
    only_when_charging: boolean;
    only_on_unmetered_wifi: boolean;
    max_thermal_level: number;
    is_charging: boolean;
    is_unmetered_wifi: boolean;
    current_thermal_level: number;
    base_fee_micro: number;
    reputation_score: number;
    is_active: boolean;
  }

  interface BidRequestUI {
    bid_id: string;
    requester_node_id: string;
    min_cpu_cores: number;
    min_ram_mb: number;
    requires_gpu: boolean;
    max_budget_micro: number;
    privacy_tier: string;
    max_latency_rtt_ms: number;
  }

  interface MatchedContractUI {
    lease_id: string;
    offer_id: string;
    bid_id: string;
    provider_node_id: string;
    requester_node_id: string;
    max_budget_micro: number;
    provider_signature: string;
    requester_signature: string;
    matched_at_unix_ms: number;
  }

  interface JournalTxUI {
    tx_id: string;
    timestamp_unix_ms: number;
    memo: string;
    debit_account: string;
    debit_amount_micro: number;
    credit_account: string;
    credit_amount_micro: number;
  }

  // Reactive state
  let balances = $state<AccountBalances>({
    available_micro: 85_400_000, // 85.400000 UMCF
    escrow_locked_micro: 25_000_000, // 25.000000 UMCF
    provider_stake_micro: 50_000_000, // 50.000000 UMCF
    slashed_pool_micro: 0,
  });

  // Contribute Device settings
  let contributeActive = $state(true);
  let onlyWhenCharging = $state(true);
  let onlyOnWifi = $state(true);
  let maxThermalLevel = $state(2); // Serious
  let baseTaskFeeMicro = $state(100);
  let isDeviceCharging = $state(true);
  let isDeviceOnWifi = $state(true);
  let currentThermal = $state(0); // Nominal

  let selectedTab = $state<'asks' | 'bids' | 'contracts' | 'ledger'>('asks');
  let exportStatus = $state<string | null>(null);

  // Sample order book data
  let asks = $state<AskOfferUI[]>([
    {
      offer_id: 'ask-desktop-4090',
      provider_node_id: 'did:umcf:desktop_rtx4090',
      cpu_cores: 16,
      ram_mb: 65536,
      gpu_backend: 'NVIDIA RTX 4090 (24 GB)',
      vram_mb: 24576,
      npu_tops: 0,
      only_when_charging: false,
      only_on_unmetered_wifi: false,
      max_thermal_level: 3,
      is_charging: true,
      is_unmetered_wifi: true,
      current_thermal_level: 1,
      base_fee_micro: 100,
      reputation_score: 985,
      is_active: true,
    },
    {
      offer_id: 'ask-phone-pixel9',
      provider_node_id: 'did:umcf:phone_pixel9',
      cpu_cores: 8,
      ram_mb: 16384,
      gpu_backend: 'Mali-G715 (Vulkan)',
      vram_mb: 8192,
      npu_tops: 45,
      only_when_charging: true,
      only_on_unmetered_wifi: true,
      max_thermal_level: 2,
      is_charging: true,
      is_unmetered_wifi: true,
      current_thermal_level: 0,
      base_fee_micro: 50,
      reputation_score: 960,
      is_active: true,
    },
  ]);

  let bids = $state<BidRequestUI[]>([
    {
      bid_id: 'bid-ai-rag-batch',
      requester_node_id: 'did:umcf:macbook_client',
      min_cpu_cores: 8,
      min_ram_mb: 16384,
      requires_gpu: true,
      max_budget_micro: 15_000_000,
      privacy_tier: 'TrustedDevices',
      max_latency_rtt_ms: 100,
    },
    {
      bid_id: 'bid-video-4k-render',
      requester_node_id: 'did:umcf:studio_workstation',
      min_cpu_cores: 16,
      min_ram_mb: 32768,
      requires_gpu: true,
      max_budget_micro: 50_000_000,
      privacy_tier: 'PublicMarket',
      max_latency_rtt_ms: 250,
    },
  ]);

  let contracts = $state<MatchedContractUI[]>([
    {
      lease_id: 'lease-rag-001',
      offer_id: 'ask-desktop-4090',
      bid_id: 'bid-ai-rag-batch',
      provider_node_id: 'did:umcf:desktop_rtx4090',
      requester_node_id: 'did:umcf:macbook_client',
      max_budget_micro: 15_000_000,
      provider_signature: '7a8f9b...e21c',
      requester_signature: '3d4c1a...99ba',
      matched_at_unix_ms: 1731322100000,
    },
  ]);

  let ledgerEntries = $state<JournalTxUI[]>([
    {
      tx_id: 'tx-001',
      timestamp_unix_ms: 1731322000000,
      memo: 'Initial developer token grant',
      debit_account: 'treasury:mint',
      debit_amount_micro: 100_000_000,
      credit_account: 'available:self',
      credit_amount_micro: 100_000_000,
    },
    {
      tx_id: 'tx-002',
      timestamp_unix_ms: 1731322050000,
      memo: 'Deposit collateral stake for worker node',
      debit_account: 'available:self',
      debit_amount_micro: 50_000_000,
      credit_account: 'stake:self',
      credit_amount_micro: 50_000_000,
    },
    {
      tx_id: 'tx-003',
      timestamp_unix_ms: 1731322100000,
      memo: 'Lock escrow budget for job-ai-rag-batch',
      debit_account: 'available:self',
      debit_amount_micro: 15_000_000,
      credit_account: 'escrow:job-ai-rag-batch',
      credit_amount_micro: 15_000_000,
    },
    {
      tx_id: 'tx-004',
      timestamp_unix_ms: 1731322150000,
      memo: 'Receipt settlement seq 1 for job-ai-rag-batch',
      debit_account: 'escrow:job-ai-rag-batch',
      debit_amount_micro: 4_600_000,
      credit_account: 'available:desktop_rtx4090',
      credit_amount_micro: 4_600_000,
    },
  ]);

  function formatCredits(micro: number): string {
    return (micro / 1_000_000).toFixed(6);
  }

  function toggleContribute() {
    contributeActive = !contributeActive;
  }

  function simulateUnplugCharger() {
    isDeviceCharging = !isDeviceCharging;
  }

  function simulateToggleWifi() {
    isDeviceOnWifi = !isDeviceOnWifi;
  }

  function exportLedgerJson() {
    const data = JSON.stringify(ledgerEntries, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `umcf-ledger-export-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    exportStatus = 'JSON exported successfully';
    setTimeout(() => { exportStatus = null; }, 3000);
  }

  function exportLedgerCsv() {
    const header = 'tx_id,timestamp_unix_ms,memo,debit_account,debit_credits,credit_account,credit_credits\n';
    const rows = ledgerEntries
      .map(
        (e) =>
          `"${e.tx_id}",${e.timestamp_unix_ms},"${e.memo}","${e.debit_account}",${(
            e.debit_amount_micro / 1_000_000
          ).toFixed(6)},"${e.credit_account}",${(e.credit_amount_micro / 1_000_000).toFixed(6)}`
      )
      .join('\n');
    const blob = new Blob([header + rows], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `umcf-ledger-export-${Date.now()}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    exportStatus = 'CSV exported successfully';
    setTimeout(() => { exportStatus = null; }, 3000);
  }
</script>

<div class="space-y-6">
  <!-- Balance Overview Header -->
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
    <div class="bg-gray-800 border border-gray-700 rounded-lg p-5">
      <div class="text-xs uppercase font-semibold text-gray-400">Available Balance</div>
      <div class="text-2xl font-bold text-emerald-400 mt-1">
        {formatCredits(balances.available_micro)} <span class="text-sm font-normal text-gray-400">UMCF</span>
      </div>
      <div class="text-xs text-gray-400 mt-1">{balances.available_micro.toLocaleString()} micro-credits</div>
    </div>

    <div class="bg-gray-800 border border-gray-700 rounded-lg p-5">
      <div class="text-xs uppercase font-semibold text-gray-400">Escrow Locked</div>
      <div class="text-2xl font-bold text-amber-400 mt-1">
        {formatCredits(balances.escrow_locked_micro)} <span class="text-sm font-normal text-gray-400">UMCF</span>
      </div>
      <div class="text-xs text-gray-400 mt-1">In-flight job guarantees</div>
    </div>

    <div class="bg-gray-800 border border-gray-700 rounded-lg p-5">
      <div class="text-xs uppercase font-semibold text-gray-400">Provider Stake Bond</div>
      <div class="text-2xl font-bold text-blue-400 mt-1">
        {formatCredits(balances.provider_stake_micro)} <span class="text-sm font-normal text-gray-400">UMCF</span>
      </div>
      <div class="text-xs text-gray-400 mt-1">Collateral for Byzantine slashing</div>
    </div>

    <div class="bg-gray-800 border border-gray-700 rounded-lg p-5">
      <div class="text-xs uppercase font-semibold text-gray-400">Slashed Penalty Pool</div>
      <div class="text-2xl font-bold text-purple-400 mt-1">
        {formatCredits(balances.slashed_pool_micro)} <span class="text-sm font-normal text-gray-400">UMCF</span>
      </div>
      <div class="text-xs text-gray-400 mt-1">Forfeited Byzantine deposits</div>
    </div>
  </div>

  <!-- Contribute Device Safety Card -->
  <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <div>
        <h3 class="text-lg font-bold text-gray-100 flex items-center gap-2">
          <span>Contribute My Device (Earn Compute Credits)</span>
          {#if contributeActive}
            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-emerald-900 text-emerald-300">
              Active Provider
            </span>
          {:else}
            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-gray-700 text-gray-300">
              Paused
            </span>
          {/if}
        </h3>
        <p class="text-sm text-gray-400 mt-1">
          Safely lend idle CPU, GPU, NPU, and storage to earn credits with strict local thermal and battery guards.
        </p>
      </div>

      <button
        onclick={toggleContribute}
        class="px-4 py-2 rounded font-medium text-sm transition-colors {contributeActive ? 'bg-red-600 hover:bg-red-700 text-white' : 'bg-emerald-600 hover:bg-emerald-700 text-white'}"
      >
        {contributeActive ? 'Pause Device Contribution' : 'Enable Device Contribution'}
      </button>
    </div>

    <div class="mt-6 pt-5 border-t border-gray-700 grid grid-cols-1 md:grid-cols-3 gap-6">
      <div class="space-y-3">
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={onlyWhenCharging} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500" />
          <span class="text-sm text-gray-300 font-medium">Only when connected to AC charger</span>
        </label>
        <div class="text-xs text-gray-400 flex items-center gap-2 pl-6">
          <span>Current AC state:</span>
          <span class="font-semibold {isDeviceCharging ? 'text-emerald-400' : 'text-red-400'}">
            {isDeviceCharging ? 'Plugged In (AC)' : 'Discharging (Battery)'}
          </span>
          <button onclick={simulateUnplugCharger} class="text-xs underline text-blue-400 hover:text-blue-300">
            [Toggle]
          </button>
        </div>
      </div>

      <div class="space-y-3">
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={onlyOnWifi} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500" />
          <span class="text-sm text-gray-300 font-medium">Only on unmetered Wi-Fi / LAN</span>
        </label>
        <div class="text-xs text-gray-400 flex items-center gap-2 pl-6">
          <span>Network connection:</span>
          <span class="font-semibold {isDeviceOnWifi ? 'text-emerald-400' : 'text-amber-400'}">
            {isDeviceOnWifi ? 'Unmetered Wi-Fi' : 'Cellular / Metered'}
          </span>
          <button onclick={simulateToggleWifi} class="text-xs underline text-blue-400 hover:text-blue-300">
            [Toggle]
          </button>
        </div>
      </div>

      <div class="space-y-2">
        <div class="flex justify-between text-sm">
          <span class="text-gray-300 font-medium">Thermal throttle ceiling:</span>
          <span class="font-semibold text-emerald-400">
            {maxThermalLevel === 0 ? 'Nominal' : maxThermalLevel === 1 ? 'Fair' : maxThermalLevel === 2 ? 'Serious' : 'Severe'}
          </span>
        </div>
        <input
          type="range"
          min="0"
          max="3"
          bind:value={maxThermalLevel}
          class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
        />
        <div class="text-xs text-gray-400">Current thermal level: Nominal (34°C)</div>
      </div>
    </div>
  </div>

  <!-- Order Book & Ledger Tabs -->
  <div class="bg-gray-800 border border-gray-700 rounded-lg overflow-hidden">
    <div class="flex border-b border-gray-700 bg-gray-850 px-4 pt-3 gap-2">
      <button
        onclick={() => { selectedTab = 'asks'; }}
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {selectedTab === 'asks' ? 'border-emerald-500 text-emerald-400 font-semibold' : 'border-transparent text-gray-400 hover:text-gray-200'}"
      >
        Active Asks ({asks.length})
      </button>

      <button
        onclick={() => { selectedTab = 'bids'; }}
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {selectedTab === 'bids' ? 'border-amber-500 text-amber-400 font-semibold' : 'border-transparent text-gray-400 hover:text-gray-200'}"
      >
        Workload Bids ({bids.length})
      </button>

      <button
        onclick={() => { selectedTab = 'contracts'; }}
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {selectedTab === 'contracts' ? 'border-blue-500 text-blue-400 font-semibold' : 'border-transparent text-gray-400 hover:text-gray-200'}"
      >
        Matched Contracts ({contracts.length})
      </button>

      <button
        onclick={() => { selectedTab = 'ledger'; }}
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {selectedTab === 'ledger' ? 'border-purple-500 text-purple-400 font-semibold' : 'border-transparent text-gray-400 hover:text-gray-200'}"
      >
        Double-Entry Journal ({ledgerEntries.length})
      </button>
    </div>

    <!-- Tab 1: Active Asks -->
    {#if selectedTab === 'asks'}
      <div class="p-5 overflow-x-auto">
        <table class="w-full text-left text-sm text-gray-300">
          <thead class="text-xs uppercase bg-gray-900/60 text-gray-400">
            <tr>
              <th class="p-3">Offer ID / Provider</th>
              <th class="p-3">Hardware Profile</th>
              <th class="p-3">Safety Guards</th>
              <th class="p-3">Reputation</th>
              <th class="p-3">Base Fee</th>
              <th class="p-3">Status</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-700">
            {#each asks as ask}
              <tr class="hover:bg-gray-750">
                <td class="p-3">
                  <div class="font-semibold text-gray-200">{ask.offer_id}</div>
                  <div class="text-xs text-gray-400 font-mono">{ask.provider_node_id}</div>
                </td>
                <td class="p-3">
                  <div>{ask.cpu_cores} cores · {Math.round(ask.ram_mb / 1024)} GB RAM</div>
                  <div class="text-xs text-gray-400">
                    {ask.gpu_backend ?? 'CPU-only'} {ask.npu_tops > 0 ? `· ${ask.npu_tops} TOPS NPU` : ''}
                  </div>
                </td>
                <td class="p-3 text-xs">
                  <div class="flex items-center gap-1">
                    <span class="w-2 h-2 rounded-full {ask.is_charging ? 'bg-emerald-400' : 'bg-red-400'}"></span>
                    <span>{ask.only_when_charging ? 'Requires AC' : 'Any power'}</span>
                  </div>
                  <div class="flex items-center gap-1 mt-0.5">
                    <span class="w-2 h-2 rounded-full {ask.is_unmetered_wifi ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
                    <span>{ask.only_on_unmetered_wifi ? 'Wi-Fi only' : 'Any net'}</span>
                  </div>
                </td>
                <td class="p-3">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-blue-900/60 text-blue-300">
                    {(ask.reputation_score / 10).toFixed(1)}%
                  </span>
                </td>
                <td class="p-3 font-mono text-emerald-400">
                  {formatCredits(ask.base_fee_micro)} UMCF
                </td>
                <td class="p-3">
                  {#if ask.is_active && (!ask.only_when_charging || ask.is_charging) && (!ask.only_on_unmetered_wifi || ask.is_unmetered_wifi)}
                    <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-emerald-900 text-emerald-300">
                      Eligible
                    </span>
                  {:else}
                    <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-amber-900 text-amber-300">
                      Constraint Paused
                    </span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <!-- Tab 2: Workload Bids -->
    {#if selectedTab === 'bids'}
      <div class="p-5 overflow-x-auto">
        <table class="w-full text-left text-sm text-gray-300">
          <thead class="text-xs uppercase bg-gray-900/60 text-gray-400">
            <tr>
              <th class="p-3">Bid ID / Requester</th>
              <th class="p-3">Min Hardware</th>
              <th class="p-3">Privacy Tier</th>
              <th class="p-3">Max Latency</th>
              <th class="p-3">Budget Cap</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-700">
            {#each bids as bid}
              <tr class="hover:bg-gray-750">
                <td class="p-3">
                  <div class="font-semibold text-gray-200">{bid.bid_id}</div>
                  <div class="text-xs text-gray-400 font-mono">{bid.requester_node_id}</div>
                </td>
                <td class="p-3">
                  <div>{bid.min_cpu_cores} cores · {Math.round(bid.min_ram_mb / 1024)} GB RAM</div>
                  <div class="text-xs text-gray-400">{bid.requires_gpu ? 'GPU Required' : 'CPU acceptable'}</div>
                </td>
                <td class="p-3">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-purple-900/60 text-purple-300">
                    {bid.privacy_tier}
                  </span>
                </td>
                <td class="p-3 text-gray-300 font-mono">
                  &lt; {bid.max_latency_rtt_ms} ms
                </td>
                <td class="p-3 font-mono text-amber-400 font-semibold">
                  {formatCredits(bid.max_budget_micro)} UMCF
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <!-- Tab 3: Matched Contracts -->
    {#if selectedTab === 'contracts'}
      <div class="p-5 overflow-x-auto">
        <table class="w-full text-left text-sm text-gray-300">
          <thead class="text-xs uppercase bg-gray-900/60 text-gray-400">
            <tr>
              <th class="p-3">Lease Contract</th>
              <th class="p-3">Parties</th>
              <th class="p-3">Budget Escrow</th>
              <th class="p-3">Signatures</th>
              <th class="p-3">Status</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-700">
            {#each contracts as c}
              <tr class="hover:bg-gray-750">
                <td class="p-3">
                  <div class="font-semibold text-gray-200 font-mono text-xs">{c.lease_id}</div>
                  <div class="text-xs text-gray-400">Match: {c.offer_id} &times; {c.bid_id}</div>
                </td>
                <td class="p-3 text-xs font-mono">
                  <div>Provider: {c.provider_node_id}</div>
                  <div>Client: {c.requester_node_id}</div>
                </td>
                <td class="p-3 font-mono text-amber-400 font-semibold">
                  {formatCredits(c.max_budget_micro)} UMCF
                </td>
                <td class="p-3 text-xs">
                  <span class="inline-flex items-center px-2 py-0.5 rounded bg-emerald-900/70 text-emerald-300 font-mono">
                    Dual Ed25519 Verified
                  </span>
                </td>
                <td class="p-3">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-emerald-900 text-emerald-300">
                    Executing
                  </span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <!-- Tab 4: Double-Entry Journal -->
    {#if selectedTab === 'ledger'}
      <div class="p-5">
        <div class="flex justify-between items-center mb-4">
          <div class="text-sm text-gray-300">
            Zero-Sum Double-Entry Transaction Ledger (<span class="text-emerald-400 font-semibold">&Sigma; Debits == &Sigma; Credits</span>)
          </div>
          <div class="flex items-center gap-2">
            {#if exportStatus}
              <span class="text-xs text-emerald-400 mr-2">{exportStatus}</span>
            {/if}
            <button
              onclick={exportLedgerCsv}
              class="px-3 py-1.5 rounded bg-gray-700 hover:bg-gray-600 text-xs font-semibold text-gray-200 transition-colors"
            >
              Export CSV
            </button>
            <button
              onclick={exportLedgerJson}
              class="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-700 text-xs font-semibold text-white transition-colors"
            >
              Export JSON
            </button>
          </div>
        </div>

        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm text-gray-300 font-mono">
            <thead class="text-xs uppercase bg-gray-900/60 text-gray-400">
              <tr>
                <th class="p-3">Tx ID</th>
                <th class="p-3">Memo</th>
                <th class="p-3">Debit Leg</th>
                <th class="p-3">Credit Leg</th>
                <th class="p-3">Amount</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-700 text-xs">
              {#each ledgerEntries as tx}
                <tr class="hover:bg-gray-750">
                  <td class="p-3 text-gray-400">{tx.tx_id}</td>
                  <td class="p-3 text-gray-200 font-sans">{tx.memo}</td>
                  <td class="p-3 text-red-400">{tx.debit_account}</td>
                  <td class="p-3 text-emerald-400">{tx.credit_account}</td>
                  <td class="p-3 text-gray-200 font-semibold">{formatCredits(tx.debit_amount_micro)} UMCF</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
</div>
