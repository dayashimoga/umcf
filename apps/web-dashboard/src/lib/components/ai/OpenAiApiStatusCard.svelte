<script lang="ts">
  interface Props {
    endpointUrl?: string;
    privacyMode?: 'strict-local' | 'trusted-devices' | 'cluster-encrypted';
    activeModelsCount?: number;
    totalRequestsServed?: number;
    avgLatencyMs?: number;
    tokensPerSec?: number;
  }

  let {
    endpointUrl = 'http://127.0.0.1:11435/v1',
    privacyMode = 'strict-local',
    activeModelsCount = 2,
    totalRequestsServed = 142,
    avgLatencyMs = 28.4,
    tokensPerSec = 34.2
  }: Props = $props();

  let activeTab = $state<'curl' | 'python' | 'typescript'>('curl');
  let copied = $state(false);

  const curlSnippet = $derived(`curl ${endpointUrl}/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "X-UMCF-Privacy-Mode: ${privacyMode}" \\
  -d '{
    "model": "Llama-3-8B-Instruct-Q4",
    "messages": [{"role": "user", "content": "Hello, private assistant!"}]
  }'`);

  const pythonSnippet = $derived(`from openai import OpenAI

client = OpenAI(
    base_url="${endpointUrl}",
    api_key="umcf-local-session",
    default_headers={"X-UMCF-Privacy-Mode": "${privacyMode}"}
)

response = client.chat.completions.create(
    model="Llama-3-8B-Instruct-Q4",
    messages=[{"role": "user", "content": "Hello from Python!"}]
)
print(response.choices[0].message.content)`);

  const tsSnippet = $derived(`import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: '${endpointUrl}',
  apiKey: 'umcf-local-session',
  defaultHeaders: { 'X-UMCF-Privacy-Mode': '${privacyMode}' }
});

const completion = await client.chat.completions.create({
  model: 'Llama-3-8B-Instruct-Q4',
  messages: [{ role: 'user', content: 'Hello from TypeScript!' }],
});
console.log(completion.choices[0].message.content);`);

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // Fallback
    }
  }

  const activeSnippet = $derived(
    activeTab === 'curl' ? curlSnippet : activeTab === 'python' ? pythonSnippet : tsSnippet
  );
</script>

<div class="bg-slate-900 border border-slate-800 rounded-2xl p-6 shadow-xl text-slate-100 max-w-4xl mx-auto">
  <!-- Header: API Status & Privacy Mode -->
  <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 pb-5 border-b border-slate-800">
    <div class="flex items-center gap-3">
      <div class="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-xl text-emerald-400 text-2xl">
        ⚡
      </div>
      <div>
        <h2 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
          <span>OpenAI-Compatible Local API</span>
          <span class="text-xs px-2.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono">
            Active: 127.0.0.1:11435
          </span>
        </h2>
        <p class="text-sm text-slate-400">
          Drop-in replacement for OpenAI SDKs • Data Sovereignty: <span class="text-emerald-400 font-semibold">{privacyMode}</span>
        </p>
      </div>
    </div>

    <!-- Live Performance Metrics -->
    <div class="flex items-center gap-3 bg-slate-950/60 border border-slate-800/80 px-4 py-2 rounded-xl">
      <div class="text-right">
        <div class="text-[11px] text-slate-400 uppercase tracking-wider">TPS</div>
        <div class="text-sm font-bold text-indigo-400">{tokensPerSec.toFixed(1)} tok/s</div>
      </div>
      <div class="w-px h-7 bg-slate-800"></div>
      <div class="text-right">
        <div class="text-[11px] text-slate-400 uppercase tracking-wider">Avg Latency</div>
        <div class="text-sm font-bold text-emerald-400">{avgLatencyMs.toFixed(1)} ms</div>
      </div>
      <div class="w-px h-7 bg-slate-800"></div>
      <div class="text-right">
        <div class="text-[11px] text-slate-400 uppercase tracking-wider">Requests</div>
        <div class="text-sm font-bold text-slate-200">{totalRequestsServed}</div>
      </div>
    </div>
  </div>

  <!-- Code Snippets Section -->
  <div class="mt-6">
    <div class="flex items-center justify-between mb-3">
      <!-- Language Tabs -->
      <div class="flex items-center gap-1 bg-slate-950 p-1 rounded-xl border border-slate-800">
        <button
          type="button"
          onclick={() => (activeTab = 'curl')}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors {activeTab === 'curl'
            ? 'bg-indigo-600 text-white'
            : 'text-slate-400 hover:text-white'}"
        >
          cURL
        </button>
        <button
          type="button"
          onclick={() => (activeTab = 'python')}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors {activeTab === 'python'
            ? 'bg-indigo-600 text-white'
            : 'text-slate-400 hover:text-white'}"
        >
          Python (openai>=1.0)
        </button>
        <button
          type="button"
          onclick={() => (activeTab = 'typescript')}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors {activeTab === 'typescript'
            ? 'bg-indigo-600 text-white'
            : 'text-slate-400 hover:text-white'}"
        >
          TypeScript (openai>=4.0)
        </button>
      </div>

      <!-- Copy Action -->
      <button
        type="button"
        onclick={() => copyToClipboard(activeSnippet)}
        class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 active:bg-slate-900 border border-slate-700 rounded-lg text-xs font-semibold text-slate-200 flex items-center gap-1.5 transition-colors"
      >
        <span>{copied ? '✓ Copied' : '📋 Copy Code'}</span>
      </button>
    </div>

    <!-- Code Block -->
    <pre class="bg-slate-950 border border-slate-800 p-4 rounded-xl text-xs font-mono text-indigo-200 overflow-x-auto leading-relaxed">
      <code>{activeSnippet}</code>
    </pre>
  </div>
</div>

