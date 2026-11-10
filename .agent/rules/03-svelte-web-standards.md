---
trigger: always_on
description: Svelte 5 runes, TypeScript strict mode, accessibility, real-time telemetry, and Cloudflare integration standards for UMCF web dashboard.
---

# 03 - Svelte 5 & Web Dashboard Standards (UMCF)

All web applications, control-plane dashboards, and browser client surfaces in the UMCF workspace must be built with **Svelte 5** and **TypeScript (strict mode)**, adhering to the standards below.

---

## 1. Svelte 5 Runes Discipline (No Legacy Reactivity)

1. **Pure Svelte 5 Runes**:
   - Use Svelte 5 runes exclusively across `.svelte` and `.svelte.ts` modules:
     - `$state()` and `$state.raw()` (use `$state.raw` for high-frequency immutable telemetry snapshots or large topology graphs to avoid deep proxy overhead)
     - `$derived()` and `$derived.by()` for computed projections
     - `$effect()` sparingly—only for synchronizing with imperative DOM APIs (Canvas/WebGL/WebGPU, WebRTC/WebTransport media elements, chart libraries), never for synchronizing one `$state` into another `$state`
     - `$props()` and `$bindable()` with explicit TypeScript interfaces
2. **Forbidden Legacy Syntax**:
   - Do not use Svelte 3/4 `export let`, `$:` reactive statements, `createEventDispatcher`, `<slot>`, or `svelte/store` (`writable`/`derived`) in new components. Use callback props and `{#snippet ...}` / `{@render ...}` tags instead.

---

## 2. TypeScript Strict Mode & Schema Validation

1. **Compiler Strictness**:
   - `tsconfig.json` must enable `"strict": true`, `"noUncheckedIndexedAccess": true`, `"exactOptionalPropertyTypes": true`, `"noImplicitReturns": true`, and `"noFallthroughCasesInSwitch": true`.
   - Zero `any` types (`@typescript-eslint/no-explicit-any: "error"`).
2. **Runtime Boundary Validation**:
   - Every payload arriving over HTTP, WebSocket, WebTransport, or `localStorage` must be validated against shared runtime schemas (matching the Rust wire contracts) before entering reactive stores.

---

## 3. Real-Time Telemetry & WebTransport / WebSocket Resilience

1. **Deterministic Connection State Machine**:
   - Real-time dashboard streams must explicitly track connection states (`Connecting`, `Connected`, `Degraded`, `Reconnecting`, `Disconnected`) with exponential backoff + jitter and sequence-number gap detection.
2. **Backpressure & Frame Rate Governance**:
   - Coalesce incoming node heartbeats, token generation streams, and cluster topology diffs using `requestAnimationFrame` batching or `$state.raw` ring buffers so rapid mesh updates never freeze the browser main thread.
3. **Interactive Control Loop**:
   - Every dashboard control (node drain, workload migration, model shard pinning, capability revocation, QR enrollment generator, emergency mesh halt) must send an authenticated command, track in-flight request status, handle optimistic/confirmed state updates, and surface server-side errors clearly.

---

## 4. Accessibility (WCAG 2.1 AA) & UX Completeness

1. **Semantic HTML & ARIA**:
   - Use semantic elements (`<nav>`, `<main>`, `<section>`, `<dialog>`, `<button>`) and proper ARIA attributes (`aria-live="polite"` for status updates, `aria-busy`, `role="status"`, `aria-label` on icon-only controls).
2. **Full Keyboard Navigation**:
   - Every interactive widget, topology node inspector, modal dialog, and table filter must be operable via keyboard (`Tab`, `Shift+Tab`, `Enter`, `Space`, `Escape`, arrow keys) with visible `:focus-visible` outlines.
3. **Color Contrast & Reduced Motion**:
   - Maintain >= 4.5:1 contrast ratio for normal text and respect `@media (prefers-reduced-motion: reduce)` for topology animations and pulse indicators.

---

## 5. Cloudflare Workers / Durable Objects & Cloud Run Parity

1. **Control Plane Edge Compatibility**:
   - Web and edge worker code targeting Cloudflare Pages/Workers must use Web-standard APIs (`fetch`, `WebSocketPair`, `crypto.subtle`, Durable Object transactional storage / SQLite backend) without Node.js-only native C++ addons.
2. **Portable Fallback Parity**:
   - Control plane API contracts must remain 100% compatible between the primary Cloudflare Workers + Durable Objects deployment and the self-hosted Cloud Run / Podman container fallback.

---

## 6. Automated Verification (`Vitest` & `Playwright`)

1. **Unit & Component Tests (`Vitest`)**:
   - Test all `.svelte.ts` state stores, protocol parsers, topology graph reducers, and UI components with `vitest` and `@testing-library/svelte` achieving >= 95% coverage.
2. **End-to-End Integration Tests (`Playwright`)**:
   - Verify complete user journeys in headless Chromium/Firefox/WebKit against a live local control-plane instance, asserting zero console errors, zero uncaught page exceptions, and real round-trip state mutations.
