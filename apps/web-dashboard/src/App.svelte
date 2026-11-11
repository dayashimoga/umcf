<script lang="ts">
  // UMCF Svelte 5 Web Administration & Control Dashboard
  import { createClusterDashboardState, PROTOCOL_VERSION } from "./cluster-store.svelte.js";

  // Component imports across all domain capabilities
  import WorkflowTemplateGalleryCard from "./lib/components/developer/WorkflowTemplateGalleryCard.svelte";
  import ApiKeyManagerCard from "./lib/components/developer/ApiKeyManagerCard.svelte";
  import StorageQuotaCard from "./lib/components/storage/StorageQuotaCard.svelte";
  import OfflinePinManagerCard from "./lib/components/storage/OfflinePinManagerCard.svelte";
  import ErasureShardMatrixCard from "./lib/components/storage/ErasureShardMatrixCard.svelte";
  import MarketplaceOrderBookCard from "./lib/components/market/MarketplaceOrderBookCard.svelte";
  import CryptographicReceiptExplorerCard from "./lib/components/market/CryptographicReceiptExplorerCard.svelte";
  import BrowserStreamPlayer from "./lib/components/stream/BrowserStreamPlayer.svelte";
  import HostStreamSettingsCard from "./lib/components/stream/HostStreamSettingsCard.svelte";
  import TranscodePipelineView from "./lib/components/media/TranscodePipelineView.svelte";

  const dashboard = createClusterDashboardState("umcf-edge-mesh");

  // Navigation state
  type NavRoute = 'overview' | 'workflows' | 'ai' | 'gpu' | 'media' | 'stream' | 'storage' | 'market' | 'keys';
  let currentRoute = $state<NavRoute>('overview');

  // Connection & Node Governance state
  let isWsConnected = $state(true);
  let localRole = $state(dashboard.localRole);
  let sharingEnabled = $state(dashboard.sharingEnabled);
  let privateAiMode = $state(dashboard.privateAiLocalOnly);

  // Storage sub-tab
  let storageTab = $state<'pins' | 'erasure' | 'quota'>('pins');
  // Stream sub-tab
  let streamTab = $state<'player' | 'host'>('player');
  // Market sub-tab
  let marketTab = $state<'orderbook' | 'receipts'>('orderbook');

  function cycleRole() {
    const next = localRole === "Hybrid" ? "Master" : localRole === "Master" ? "Worker" : "Hybrid";
    localRole = dashboard.setRole(next);
  }

  function toggleSharing() {
    sharingEnabled = !sharingEnabled;
    dashboard.updateConsent({ enabled: sharingEnabled });
  }

  function togglePrivateAi() {
    privateAiMode = !privateAiMode;
    dashboard.setPrivateAiMode(privateAiMode);
  }

  function simulateReconnect() {
    isWsConnected = false;
    setTimeout(() => {
      isWsConnected = true;
    }, 1200);
  }
</script>

<div class="min-h-screen bg-gray-950 text-gray-100 flex flex-col font-sans">
  <!-- Top Navigation Header -->
  <header class="bg-gray-900 border-b border-gray-800 sticky top-0 z-50 px-6 py-3.5 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-indigo-600 flex items-center justify-center font-bold text-white shadow-lg shadow-indigo-600/30">
        U
      </div>
      <div>
        <div class="flex items-center gap-2">
          <span class="font-bold text-lg tracking-tight text-white">UMCF Control Plane</span>
          <span class="text-xs px-2 py-0.5 rounded bg-gray-800 text-gray-300 font-mono">v{PROTOCOL_VERSION}</span>
        </div>
        <div class="text-xs text-gray-400">Universal Mobile Compute Fabric Dashboard</div>
      </div>
    </div>

    <!-- Cluster Status Banner & Health -->
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2 bg-gray-850 border border-gray-800 px-3 py-1.5 rounded-lg text-xs">
        <span class="w-2.5 h-2.5 rounded-full {isWsConnected ? 'bg-emerald-400 animate-pulse' : 'bg-red-500'}"></span>
        <span class="font-medium {isWsConnected ? 'text-emerald-400' : 'text-red-400'}">
          {isWsConnected ? 'Gateway WebSocket: Connected' : 'Reconnecting...'}
        </span>
        <button
          onclick={simulateReconnect}
          class="text-gray-500 hover:text-gray-300 ml-1 underline"
          title="Simulate network reconnect"
        >
          [Probe]
        </button>
      </div>

      <div class="hidden md:flex items-center gap-3 text-xs text-gray-400">
        <div>Fabric: <span class="text-gray-200 font-semibold">4 Nodes</span></div>
        <div>Cores: <span class="text-gray-200 font-semibold">46 Physical</span></div>
        <div>RAM: <span class="text-gray-200 font-semibold">112 GB</span></div>
      </div>
    </div>
  </header>

  <!-- Route Navigation Bar -->
  <nav class="bg-gray-900/90 backdrop-blur border-b border-gray-800 px-6 overflow-x-auto flex gap-1">
    <button
      onclick={() => { currentRoute = 'overview'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'overview' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Cluster Overview
    </button>

    <button
      onclick={() => { currentRoute = 'workflows'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'workflows' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      DAG Workflows
    </button>

    <button
      onclick={() => { currentRoute = 'ai'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'ai' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Distributed AI
    </button>

    <button
      onclick={() => { currentRoute = 'gpu'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'gpu' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      GPU Rendering
    </button>

    <button
      onclick={() => { currentRoute = 'media'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'media' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Media Transcoding
    </button>

    <button
      onclick={() => { currentRoute = 'stream'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'stream' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Low-Latency Stream
    </button>

    <button
      onclick={() => { currentRoute = 'storage'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'storage' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Distributed Storage
    </button>

    <button
      onclick={() => { currentRoute = 'market'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'market' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      Marketplace & Ledger
    </button>

    <button
      onclick={() => { currentRoute = 'keys'; }}
      class="px-3.5 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {currentRoute === 'keys' ? 'border-indigo-500 text-indigo-400 bg-gray-850/50' : 'border-transparent text-gray-400 hover:text-gray-200'}"
    >
      API Keys & SDK
    </button>
  </nav>

  <!-- Main Content Area -->
  <main class="flex-1 p-6 max-w-7xl mx-auto w-full">
    <!-- View 1: Overview & Node Governance -->
    {#if currentRoute === 'overview'}
      <div class="space-y-6">
        <!-- Node Governance Card -->
        <div class="bg-gray-900 border border-gray-800 rounded-lg p-5">
          <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
            <div>
              <h2 class="text-base font-bold text-white flex items-center gap-2">
                <span>Local Node Governance & Consent</span>
                <span class="text-xs px-2 py-0.5 rounded bg-indigo-950 text-indigo-300 font-mono">
                  Role: {localRole}
                </span>
              </h2>
              <p class="text-xs text-gray-400 mt-1">
                Configure participation boundaries, hardware access consent, and privacy shields for this host.
              </p>
            </div>

            <button
              onclick={cycleRole}
              class="px-3.5 py-1.5 rounded bg-indigo-600 hover:bg-indigo-700 text-xs font-semibold text-white transition-colors"
            >
              Transition Role ({localRole})
            </button>
          </div>

          <div class="mt-4 pt-4 border-t border-gray-800 grid grid-cols-1 sm:grid-cols-2 gap-4">
            <button
              type="button"
              onclick={toggleSharing}
              class="text-left p-3.5 rounded-lg border transition-colors {sharingEnabled ? 'bg-emerald-950/30 border-emerald-800/60' : 'bg-gray-850 border-gray-750'}"
            >
              <div class="text-xs font-semibold {sharingEnabled ? 'text-emerald-400' : 'text-gray-300'}">
                Resource Sharing Consent
              </div>
              <div class="text-xs text-gray-400 mt-1">
                {sharingEnabled ? "Enabled — remote tasks permitted" : "Disabled — strictly local execution"}
              </div>
            </button>

            <button
              type="button"
              onclick={togglePrivateAi}
              class="text-left p-3.5 rounded-lg border transition-colors {privateAiMode ? 'bg-purple-950/30 border-purple-800/60' : 'bg-gray-850 border-gray-750'}"
            >
              <div class="text-xs font-semibold {privateAiMode ? 'text-purple-400' : 'text-gray-300'}">
                Private AI Enclave Shield
              </div>
              <div class="text-xs text-gray-400 mt-1">
                {privateAiMode ? "StrictLocalOnly — tokens never leave device" : "Cluster Distributed Layer Pipeline"}
              </div>
            </button>
          </div>
        </div>

        <!-- Enrolled Nodes Table -->
        <div class="bg-gray-900 border border-gray-800 rounded-lg overflow-hidden">
          <div class="p-4 border-b border-gray-800 flex justify-between items-center">
            <h3 class="text-sm font-bold text-white">Fabric Topology & Health (4 Active Nodes)</h3>
            <span class="text-xs text-gray-400 font-mono">mTLS 1.3 Mesh</span>
          </div>

          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs text-gray-300">
              <thead class="uppercase bg-gray-950 text-gray-400">
                <tr>
                  <th class="p-3">Node Name / DID</th>
                  <th class="p-3">Role</th>
                  <th class="p-3">Hardware Profile</th>
                  <th class="p-3">Thermal / Power</th>
                  <th class="p-3">Load</th>
                  <th class="p-3">Status</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-800">
                <tr class="hover:bg-gray-850/50">
                  <td class="p-3">
                    <div class="font-semibold text-white">Intel Ultra 5 Desktop (Local)</div>
                    <div class="text-gray-500 font-mono text-2xs">did:umcf:desktop_rtx4090</div>
                  </td>
                  <td class="p-3"><span class="px-2 py-0.5 rounded bg-indigo-900 text-indigo-300">Hybrid / Master</span></td>
                  <td class="p-3">14 Cores · 16 GB RAM · Intel Arc 130T</td>
                  <td class="p-3">Nominal (38°C) · AC Powered</td>
                  <td class="p-3">12% CPU · 38% RAM</td>
                  <td class="p-3"><span class="text-emerald-400 font-semibold">Online</span></td>
                </tr>
                <tr class="hover:bg-gray-850/50">
                  <td class="p-3">
                    <div class="font-semibold text-white">Google Pixel 9 Pro</div>
                    <div class="text-gray-500 font-mono text-2xs">did:umcf:phone_pixel9</div>
                  </td>
                  <td class="p-3"><span class="px-2 py-0.5 rounded bg-gray-800 text-gray-300">Worker</span></td>
                  <td class="p-3">8 Cores · 16 GB RAM · 45 TOPS NPU</td>
                  <td class="p-3">Nominal (32°C) · Charging (92%)</td>
                  <td class="p-3">4% CPU · 29% RAM</td>
                  <td class="p-3"><span class="text-emerald-400 font-semibold">Online</span></td>
                </tr>
                <tr class="hover:bg-gray-850/50">
                  <td class="p-3">
                    <div class="font-semibold text-white">MacBook Pro M3 Max</div>
                    <div class="text-gray-500 font-mono text-2xs">did:umcf:macbook_m3</div>
                  </td>
                  <td class="p-3"><span class="px-2 py-0.5 rounded bg-gray-800 text-gray-300">Worker</span></td>
                  <td class="p-3">16 Cores · 64 GB RAM · Metal 40-core</td>
                  <td class="p-3">Nominal (36°C) · AC Powered</td>
                  <td class="p-3">8% CPU · 28% RAM</td>
                  <td class="p-3"><span class="text-emerald-400 font-semibold">Online</span></td>
                </tr>
                <tr class="hover:bg-gray-850/50">
                  <td class="p-3">
                    <div class="font-semibold text-white">Ubuntu AI Rig (RTX 4090)</div>
                    <div class="text-gray-500 font-mono text-2xs">did:umcf:server_rtx4090</div>
                  </td>
                  <td class="p-3"><span class="px-2 py-0.5 rounded bg-gray-800 text-gray-300">Worker</span></td>
                  <td class="p-3">32 Cores · 128 GB RAM · RTX 4090 24GB</td>
                  <td class="p-3">Fair (52°C) · AC Powered</td>
                  <td class="p-3">22% CPU · 45% VRAM</td>
                  <td class="p-3"><span class="text-emerald-400 font-semibold">Online</span></td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    {/if}

    <!-- View 2: Workflows & DAG Manifests -->
    {#if currentRoute === 'workflows'}
      <WorkflowTemplateGalleryCard />
    {/if}

    <!-- View 3: Distributed AI & Enclave -->
    {#if currentRoute === 'ai'}
      <div class="space-y-6">
        <div class="bg-gray-900 border border-gray-800 rounded-lg p-5">
          <h2 class="text-base font-bold text-white">Distributed AI Inference Engine (llama.cpp)</h2>
          <p class="text-xs text-gray-400 mt-1">
            GGUF Model Sharding, Resumable P2P Activation Transfer, and Strict-Local-Only Privacy Enclave.
          </p>

          <div class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="p-4 rounded-lg bg-gray-850 border border-gray-800">
              <div class="text-xs font-semibold text-gray-400 uppercase">Active Model</div>
              <div class="text-sm font-bold text-white mt-1">Llama-3.2-3B-Instruct</div>
              <div class="text-xs text-indigo-400 mt-0.5 font-mono">Q4_K_M · 2.1 GB</div>
            </div>
            <div class="p-4 rounded-lg bg-gray-850 border border-gray-800">
              <div class="text-xs font-semibold text-gray-400 uppercase">KV Cache Hit Rate</div>
              <div class="text-sm font-bold text-emerald-400 mt-1">85.4%</div>
              <div class="text-xs text-gray-400 mt-0.5">TTFT Speedup: 92.1%</div>
            </div>
            <div class="p-4 rounded-lg bg-gray-850 border border-gray-800">
              <div class="text-xs font-semibold text-gray-400 uppercase">Layer Partitioning</div>
              <div class="text-sm font-bold text-white mt-1">3 Nodes (Layers 0..27)</div>
              <div class="text-xs text-gray-400 mt-0.5">Bit-Exact Parity 1.0</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- View 4: GPU Rendering -->
    {#if currentRoute === 'gpu'}
      <div class="space-y-6">
        <div class="bg-gray-900 border border-gray-800 rounded-lg p-5">
          <h2 class="text-base font-bold text-white">Tile/Frame-Partitioned Distributed Rendering</h2>
          <p class="text-xs text-gray-400 mt-1">
            WGSL Compute Shader Dispatch, Spiral Tile Partitioning, and Byzantine Canary Checksum Verification.
          </p>
          <div class="mt-4 p-4 rounded bg-gray-850 border border-gray-800 text-xs text-gray-300">
            Tile Engine Status: <span class="text-emerald-400 font-semibold">Ready</span> · 4 GPU Accelerators Enrolled
          </div>
        </div>
      </div>
    {/if}

    <!-- View 5: Media Transcoding -->
    {#if currentRoute === 'media'}
      <TranscodePipelineView />
    {/if}

    <!-- View 6: Low-Latency Streaming -->
    {#if currentRoute === 'stream'}
      <div class="space-y-4">
        <div class="flex gap-2">
          <button
            onclick={() => { streamTab = 'player'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {streamTab === 'player' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Remote Stream Player
          </button>
          <button
            onclick={() => { streamTab = 'host'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {streamTab === 'host' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Host Encoder Settings
          </button>
        </div>

        {#if streamTab === 'player'}
          <BrowserStreamPlayer />
        {:else}
          <HostStreamSettingsCard />
        {/if}
      </div>
    {/if}

    <!-- View 7: Distributed Storage -->
    {#if currentRoute === 'storage'}
      <div class="space-y-4">
        <div class="flex gap-2">
          <button
            onclick={() => { storageTab = 'pins'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {storageTab === 'pins' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Offline Pins & GC
          </button>
          <button
            onclick={() => { storageTab = 'erasure'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {storageTab === 'erasure' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Reed-Solomon RS(k,m) Shards
          </button>
          <button
            onclick={() => { storageTab = 'quota'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {storageTab === 'quota' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            CAS Quota Enforcer
          </button>
        </div>

        {#if storageTab === 'pins'}
          <OfflinePinManagerCard />
        {:else if storageTab === 'erasure'}
          <ErasureShardMatrixCard />
        {:else}
          <StorageQuotaCard />
        {/if}
      </div>
    {/if}

    <!-- View 8: Marketplace & Ledger -->
    {#if currentRoute === 'market'}
      <div class="space-y-4">
        <div class="flex gap-2">
          <button
            onclick={() => { marketTab = 'orderbook'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {marketTab === 'orderbook' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Order Book & Double-Entry Ledger
          </button>
          <button
            onclick={() => { marketTab = 'receipts'; }}
            class="px-3 py-1.5 rounded text-xs font-semibold transition-colors {marketTab === 'receipts' ? 'bg-indigo-600 text-white' : 'bg-gray-800 text-gray-300'}"
          >
            Cryptographic Receipt Explorer
          </button>
        </div>

        {#if marketTab === 'orderbook'}
          <MarketplaceOrderBookCard />
        {:else}
          <CryptographicReceiptExplorerCard />
        {/if}
      </div>
    {/if}

    <!-- View 9: API Keys & SDK -->
    {#if currentRoute === 'keys'}
      <ApiKeyManagerCard />
    {/if}
  </main>
</div>
