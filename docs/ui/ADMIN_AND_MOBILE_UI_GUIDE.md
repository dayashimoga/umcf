# UMCF User Interface Guide: Admin Web Dashboard & Multiplatform Client

This document provides architectural guidance, screen organization, state management conventions, and interaction patterns for both official UMCF user interfaces:
1. **Svelte 5 Web Administration & Operator Dashboard** (`apps/web-dashboard`, `ui/admin_web`)
2. **Flutter Cross-Platform Application** (`apps/mobile-desktop`, `ui/flutter_app` targeting Android, iOS, Windows, Linux, and macOS)

---

## 1. High-Level UI Architecture

```text
┌────────────────────────────────────────────────────────────────────────┐
│                      UMCF USER INTERFACE SURFACES                      │
├───────────────────────────────────┬────────────────────────────────────┤
│   Svelte 5 Admin Web Dashboard    │   Flutter Multiplatform Client     │
│   (Desktop Browser / Control)     │   (Android, iOS, Win, Mac, Linux)  │
├───────────────────────────────────┴────────────────────────────────────┤
│                        SHARED DOMAIN CAPABILITIES                      │
│  1. Cluster Governance & Topology   5. Media Transcode & Annex-B       │
│  2. Declarative DAG Workflows       6. Low-Latency QUIC/WebRTC Stream  │
│  3. Distributed AI & Enclave        7. Distributed Storage & Pins      │
│  4. GPU 3D Render Studio            8. Double-Entry Marketplace Ledger │
├────────────────────────────────────────────────────────────────────────┤
│                     TRANSPORT & PROTOCOL LAYER                         │
│   umcf-api-gateway (Axum 0.7 REST / OpenAPI 3.1 + WebSocket Event Ring)│
│   flutter_rust_bridge (Low-latency asynchronous FFI channels)          │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Adaptive Multiplatform Layout Guidelines

### 2.1 Screen Breakpoints
- **Compact View (`< 800px` width)**:
  - Form factor: Smartphones (portrait) and narrow desktop windows.
  - Navigation: Material 3 `NavigationBar` fixed at the bottom with touch-friendly targets ($\ge 48 \times 48\text{ dp}$).
  - Views: Single-column vertically scrollable cards with bottom sheet inspectors.
- **Expanded View (`\ge 800px` width)**:
  - Form factor: Tablets, foldables, laptops, and desktop monitors.
  - Navigation: Vertical `NavigationRail` / sidebar on the left with prominent branding and fabric connection status.
  - Views: Two-pane master-detail layouts, side-by-side telemetry grids, and resizable split views.

### 2.2 Svelte 5 Runes Discipline
- Pure Svelte 5 runes (`$state()`, `$derived()`, `$effect()`) are used exclusively.
- All high-frequency telemetry updates (e.g. 10 Hz cluster metrics, 60 Hz token streams) use `$state.raw` or batched requestAnimationFrame updates to eliminate unnecessary reactive DOM thrashing.

### 2.3 Flutter Riverpod State Management
- Architecture:
  - `bridge/`: Generated `flutter_rust_bridge` FFI bindings.
  - `domain/`: Pure Dart immutable data models and value objects.
  - `application/`: `StateNotifier` controllers managing live backend updates.
  - `presentation/`: Responsive widgets with zero direct FFI invocations in `build()`.

---

## 3. Domain Screen Matrix

| Feature Domain | Svelte 5 Component | Flutter Screen | Key Interactive Capabilities |
| :--- | :--- | :--- | :--- |
| **Cluster Overview** | `App.svelte` (Overview Tab) | `ClusterOverviewView` | Node role switching (Master/Worker/Hybrid), opt-in consent toggle, live topology list. |
| **DAG Workflows** | `WorkflowTemplateGalleryCard` | `WorkflowTemplatesScreen` | Parameter form auto-generation, DAG stage graph, lockfile receipt verification. |
| **Distributed AI** | `App.svelte` (AI Tab) | `AiEnclaveOverviewView` | GGUF model selector, StrictLocalOnly privacy shield, KV cache hit metrics. |
| **GPU Rendering** | `App.svelte` (GPU Tab) | `DistributedRenderStudioScreen` | WGSL shader dispatch, spiral tile progress compositor, Byzantine canary audit. |
| **Media Transcoding** | `TranscodePipelineView` | `TranscodeQueueScreen` | GOP segment timeline, multi-node progress, SSIM/PSNR perceptual metrics. |
| **Low-Latency Stream** | `BrowserStreamPlayer`, `HostStreamSettingsCard` | `RemoteStreamClientView` | WebRTC/QUIC canvas player, virtual gamepad/trackpad overlay, Glass-to-Glass HUD. |
| **Storage & Quota** | `OfflinePinManagerCard`, `ErasureShardMatrixCard`, `StorageQuotaCard` | `OfflinePinManagerScreen`, `StorageQuotaSheet` | CAS pin management, $RS(k,m)$ shard placement map, quota reservation slider. |
| **Marketplace & Ledger**| `MarketplaceOrderBookCard`, `CryptographicReceiptExplorerCard` | `MarketplaceOrderbookScreen`, `ReceiptExplorerScreen` | Dual-entry ledger table, CSV/JSON export, live Ask/Bid order book, dual-signature modal. |
| **API Keys & SDK** | `ApiKeyManagerCard` | `ApiKeysScreen` | Scoped Bearer capability tokens, rate limit gauges, OpenAPI spec explorer. |

---

## 4. Keyboard Shortcuts & Accessibility (WCAG 2.1 AA)

- `Tab` / `Shift+Tab`: Full sequential focus traversal across all interactive cards, tabs, and form controls.
- `Enter` / `Space`: Activate buttons, toggles, and expandable drawers.
- `Escape`: Instantly dismiss modals, bottom sheets, or close active remote streaming sessions.
- `:focus-visible`: High-contrast 2px focus ring across all components for keyboard operators.
- `prefers-reduced-motion`: Disables non-essential pulse animations on status pills and telemetry gauges.
