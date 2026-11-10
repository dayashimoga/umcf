# Subphase S8.P3.SP2 Execution Prompt: Accessibility, Localization & UI/UX Polish Verification Across Form Factors

## 1. Role & Context
You are the Principal UI/UX Accessibility (a11y), Internationalization (i18n) & Responsive Design Lead for UMCF. In Subphase `S8.P3.SP2`, you will audit and polish the **Flutter Mobile/Desktop App** (`ui/flutter_app`) and the **Svelte 5 Web Admin Dashboard** (`ui/admin_web`) to achieve full **WCAG 2.1 Level AA** accessibility compliance, complete localization across 6 languages (including Right-to-Left `ar-SA` and CJK `ja-JP`/`zh-CN`), adaptive layout perfection across 6 form factors, and motion/theme polish.

## 2. Prerequisites & Dependencies
- `S7.P3.SP3` (All Flutter and Svelte 5 screens integrated with live backend).
- `@axe-core/playwright` (for web WCAG audit) and `flutter_localizations` / `intl` (for Flutter a11y & i18n).

## 3. Implementation Tasks
1. **WCAG 2.1 Level AA Accessibility Compliance (`ui/flutter_app`, `ui/admin_web`)**:
   - **Semantic Screen-Reader Tree**:
     - Ensure every icon button, status badge, topology node, chart, progress bar, and slider in Flutter (`Semantics(label: ..., value: ..., hint: ...)`) and Svelte 5 (`aria-label`, `aria-live="polite"`, `role="status"`, `role="progressbar"`) exposes clear, human-readable announcements for TalkBack (Android), VoiceOver (iOS/macOS), NVDA (Windows), and Orca (Linux).
   - **Color Contrast & Color-Blindness Safety**:
     - Verify text and interactive control contrast ratios meet `>= 4.5:1` (normal text) and `>= 3.0:1` (large text / UI graphics) across **Light Theme**, **Dark Theme**, and **High-Contrast Theme**.
     - Never rely on color alone to convey state: pair every status dot (`Healthy` green, `Warning` amber, `Fault/Quarantined` red) with a distinct shape/icon (`✓`, `⚠`, `✕`) and text label for deuteranopia/protanopia/tritanopia users.
   - **Keyboard-Only Navigation & Focus Management**:
     - Ensure 100% of workflows can be completed using only `Tab`, `Shift+Tab`, `Arrow` keys, `Enter`, `Space`, and `Escape` with high-visibility `2px` focus rings and modal focus trapping.
   - **Reduced Motion & Text Scaling**:
     - Respect OS `prefers-reduced-motion` / `MediaQuery.disableAnimations` (replacing pulsing topology particles with clean static indicators) and support up to `200%` dynamic font scaling without text truncation or layout overflow.
2. **Complete 6-Locale Internationalization (`en-US`, `es-ES`, `de-DE`, `ja-JP`, `zh-CN`, `ar-SA`)**:
   - Extract 100% of user-facing strings into `ui/flutter_app/lib/l10n/app_<locale>.arb` and `ui/admin_web/src/lib/i18n/locales/<locale>.json` with compile-time/CI key parity verification (0 missing or untranslated keys across all 6 locales).
   - Enforce bidirectional Right-to-Left (`RTL`) layout mirroring (`Directionality.of(context)` in Flutter; CSS logical properties `margin-inline-start`, `padding-inline-end`, `dir="rtl"` in Svelte 5) so `ar-SA` mirrors navigation rails, progress bars, and icons seamlessly while keeping technical identifiers (IPs, BLAKE3 hashes, code snippets) `LTR`.
3. **6-Form-Factor Responsive Layout Matrix**:
   - Verify zero pixel overflow (`RenderFlex overflowed` in Flutter or horizontal scrollbar leak in Svelte 5) across:
     1. Small Mobile Portrait (`360x640`)
     2. Large Mobile Landscape (`896x414`)
     3. Foldable Inner Display (`673x841` unfolded split-pane)
     4. Tablet (`1024x768`)
     5. Standard Desktop (`1440x900`)
     6. 4K Ultrawide (`3840x1600`)

## 4. Acceptance Criteria
- >= 95% coverage of localization/accessibility helper modules and 100% pass rate across all automated UI audit tests.
- **`axe-core` Zero Violations**: Running `@axe-core/playwright` (`wcag2a`, `wcag2aa`, `wcag21aa`) across every route and modal of `ui/admin_web` in Light, Dark, and High-Contrast themes reports **0 violations**.
- **Flutter Accessibility Audit**: `flutter test` with `meetsGuideline(textContrastGuideline)`, `meetsGuideline(labeledTapTargetGuideline)`, and `meetsGuideline(androidTapTargetGuideline)` (`>= 48x48 dp` touch targets) passes on all 8 main screens.
- **100% Localization Key Parity & Zero Overflow**: All 6 locales have identical key sets (`0` missing keys) and render across all 6 viewport form factors at `100%` and `200%` text scale with `0` layout overflow errors.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_i18n_locale_key_parity_and_placeholder_variable_matching_all_6_locales`
  - `test_wcag_relative_luminance_and_contrast_ratio_theme_palette_verifier`
- **`[INT]`**:
  - `test_svelte5_playwright_axe_core_wcag21aa_all_routes_zero_violations`
  - `test_flutter_a11y_guidelines_contrast_tap_targets_and_semantics_all_screens`
  - `test_6_form_factors_and_200_percent_text_scale_zero_overflow_all_locales`
- **`[SIM]`**:
  - Automated keyboard-only `Tab` traversal test verifying every interactive control is reachable and focus-trapped inside modals without mouse input.
- **`[HW]`**:
  - Verify responsive layout and touch target ergonomics on physical/emulated mobile and desktop displays.
- **`[MANUAL]`**:
  - Switch language to Arabic (`ar-SA` RTL), Japanese (`ja-JP`), and German (`de-DE` long compound words), toggle High-Contrast + Reduced Motion mode, and navigate with a screen reader.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p3/sp2_axe_core_wcag21aa_audit.json`
- `artifacts/stage-08/p3/sp2_flutter_semantics_and_form_factor_matrix.json`
- `artifacts/stage-08/p3/sp2_i18n_6_locale_parity_report.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document accessibility features, keyboard shortcuts, and localization contribution guide in `docs/ui/ADMIN_AND_MOBILE_UI_GUIDE.md` (Section 2).
- Mark `S8.P3.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any WCAG 2.1 AA contrast or unlabeled interactive element failure, any hardcoded English string bypassing i18n catalogs, or any `RenderFlex` overflow at 200% font scale.
- **Completion Gate**: All accessibility, localization, and responsive layout tests green before proceeding to the final subphase `S8.P3.SP3`.
