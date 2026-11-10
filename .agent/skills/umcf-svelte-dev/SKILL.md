---
name: umcf-svelte-dev
description: >-
  Guides development of the UMCF Svelte 5 web dashboard and Cloudflare control-plane integration, including Svelte 5 runes ($state, $derived, $effect), strict TypeScript, WebSocket/WebTransport telemetry, and Vitest/Playwright testing. Use when working on apps/web_dashboard or control-plane web surfaces.
---

# UMCF Svelte 5 Web Dashboard & Control Plane UI Skill

Use this skill when building, refactoring, or testing the Svelte 5 web dashboard (`apps/web_dashboard`) and its integration with the Cloudflare Workers / Durable Objects (and Cloud Run fallback) control plane.

---

## 1. Svelte 5 Runes Architecture

1. **Reactive State Modules (`.svelte.ts`)**:
   Encapsulate domain state in typed class-based or factory-based `.svelte.ts` modules using Svelte 5 runes:
   ```ts
   // src/lib/state/mesh.svelte.ts
   import type { NodeSummary, ClusterMetrics, ConnectionState } from '$lib/types/protocol';

   export class MeshStore {
     nodes = $state.raw<ReadonlyArray<NodeSummary>>([]);
     connectionState = $state<ConnectionState>('disconnected');
     selectedNodeId = $state<string | null>(null);
     lastError = $state<string | null>(null);

     selectedNode = $derived(
       this.nodes.find((n) => n.nodeId === this.selectedNodeId) ?? null
     );

     onlineNodesCount = $derived(
       this.nodes.filter((n) => n.status === 'online').length
     );

     totalAvailableRamMb = $derived(
       this.nodes.reduce((acc, n) => acc + (n.status === 'online' ? n.availableRamMb : 0), 0)
     );
   }
   ```
2. **High-Frequency Telemetry Optimization**:
   - Use `$state.raw()` for large arrays of nodes, topology edges, and time-series metrics buffers so Svelte does not recursively wrap every nested property in a `Proxy`.
   - Batch WebSocket frame updates via a 16 ms (`requestAnimationFrame`) or 100 ms coalescing buffer before reassigning `$state.raw` references.

---

## 2. Component Authoring Standards

1. **Props & Snippets**:
   - Declare component props using `$props()` with explicit TypeScript interfaces:
     ```svelte
     <script lang="ts">
       import type { Snippet } from 'svelte';
       import type { NodeSummary } from '$lib/types/protocol';

       interface Props {
         node: NodeSummary;
         onSelect: (nodeId: string) => void;
         onDrain: (nodeId: string) => Promise<void>;
         headerBadge?: Snippet<[NodeSummary]>;
       }

       let { node, onSelect, onDrain, headerBadge }: Props = $props();
       let isDraining = $state(false);
     </script>
     ```
2. **Zero Dead Buttons or Mock Panels**:
   - Every dashboard control—node filter, search box, sort header, capability toggle, workload dispatch form, QR enrollment modal, node drain/revoke button, and telemetry time-window selector—must execute real logic and reflect live state.

---

## 3. Control Plane WebSocket & REST Client

1. **Resilient Signaling Client**:
   - Authenticate requests using signed capability tokens or session credentials.
   - Handle WebSocket reconnection with exponential backoff (`base = 500ms`, `max = 15000ms`, random jitter) and automatic state resynchronization upon reconnect.
   - Validate every incoming JSON/binary frame against strict runtime schema validators before mutating `MeshStore`.
2. **Offline / Degraded Mode Banner**:
   - When the control plane is unreachable or switching over to the Cloud Run fallback endpoint, render an explicit, accessible status banner showing connection state, fallback target, and manual **Reconnect Now** button.

---

## 4. Accessibility & Testing Requirements

1. **WCAG 2.1 AA Compliance**:
   - Include `aria-label`, `aria-live="polite"` status regions, keyboard focus traps on `<dialog>` modals, and full keyboard navigation across topology node lists.
2. **Verification Suite**:
   - **Unit/Component (`vitest`)**: Test every `.svelte.ts` store and component interaction (filtering, draining, error recovery, token stream rendering).
   - **End-to-End (`playwright`)**: Test full user flows against a running local control plane and simulated nodes.
