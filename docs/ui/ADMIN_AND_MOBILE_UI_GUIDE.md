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

- **Keyboard Navigation**:
  - `Tab` / `Shift+Tab`: Full sequential focus traversal across all interactive cards, tabs, and form controls.
  - `Enter` / `Space`: Activate buttons, toggles, dropdowns, and expandable drawers.
  - `Escape`: Instantly dismiss modals, bottom sheets, or close active remote streaming sessions.
  - `:focus-visible`: High-contrast 2px focus ring (`#58a6ff` / `#388bfd`) across all components for keyboard operators.
  - **Skip Link**: Hidden accessible skip link (`#main-content`) allowing keyboard and screen-reader users to bypass repetitive header navigation (WCAG 2.4.1).
  - **Modal Focus Trap**: Focus is strictly trapped within open dialogs and sheets until dismissed.
- **Color Contrast & Independence**:
  - Normal text contrast $\ge 4.5:1$ against background in both Dark Theme (`15.6:1`) and Light Theme (`17.5:1`).
  - High-Contrast Mode contrast $\ge 7.0:1$ (`19.5:1` yellow on black, `21.0:1` white on black).
  - UI borders and component outlines $\ge 3.0:1$ (`3.62:1` on dark background).
  - **Shape + Icon + Text**: No state is communicated via color alone (e.g. `✓ Healthy`, `⚠ Degraded/Warning`, `✕ Quarantined/Offline`).
- **Touch Target Ergonomics**:
  - All interactive touch targets in Flutter and Web are $\ge 48 \times 48\text{ dp}$ ($\ge 9.05\text{ mm}$ physical dimension), fully compliant with Android guidelines and WCAG 2.5.5.
- **Dynamic Text Scaling & Reduced Motion**:
  - Supports up to `200%` dynamic text scaling across all 6 viewports with zero text truncation or `RenderFlex` overflow errors.
  - Respects `@media (prefers-reduced-motion: reduce)` and `MediaQuery.disableAnimations` by halting pulse animations and particle emitters.

---

## 5. Internationalization (i18n) & Bidirectional (RTL) Layout

### 5.1 Supported Locales (6 Locales)
UMCF provides 100% key parity (zero missing keys) across 6 languages:
1. `en-US`: English (United States) — Base locale
2. `es-ES`: Español (Spanish)
3. `de-DE`: Deutsch (German) — Verified with long compound words (`Batterieenergieverwaltungsregler`)
4. `ja-JP`: 日本語 (Japanese) — CJK typography & compact word wrapping
5. `zh-CN`: 简体中文 (Chinese Simplified) — CJK typography & simplified characters
6. `ar-SA`: العربية (Arabic) — Right-to-Left (RTL) layout mirroring

### 5.2 Bidirectional Layout (RTL) Rules
- When `ar-SA` is selected:
  - Svelte 5 dynamically applies `dir="rtl"` to `<html>`, mirroring navigation rails, toolbars, and layout grids using CSS logical properties (`margin-inline-start`, `padding-inline-end`).
  - Flutter applies `Directionality.rtl`, mirroring `AppBar` icons, navigation bars, and progress bars.
  - Technical identifiers (IP addresses, port numbers, BLAKE3 digests, Ed25519 public keys, and code snippets) remain strictly `LTR` using `<bdi>` or `TextDirection.ltr` wrappers.

---

## 6. Six-Form-Factor Responsive Layout Matrix

| Viewport Profile | Resolution | Form Factor | Navigation Pattern | Overflow SLA |
| :--- | :--- | :--- | :--- | :--- |
| **Small Mobile Portrait** | `360 x 640` | Compact smartphones | Bottom `NavigationBar` | 0px overflow at 200% text scale |
| **Large Mobile Landscape**| `896 x 414` | Wide smartphones | Compact `NavigationRail` | 0px overflow at 200% text scale |
| **Foldable Inner** | `673 x 841` | Unfolded foldables | TwoPane Master-Detail | 0px overflow at 200% text scale |
| **Tablet** | `1024 x 768` | iPads & Android tablets | Two-Column Split Grid | 0px overflow at 200% text scale |
| **Standard Desktop** | `1440 x 900` | Laptops & monitors | Left Sidebar + Central Grid | 0px overflow at 200% text scale |
| **4K Ultrawide** | `3840 x 1600` | Ultrawide displays (2.4:1) | Multi-Panel Telemetry Grid | Auto-expanding multi-column flow |


