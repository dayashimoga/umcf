#!/usr/bin/env bash
# ==============================================================================
# S8.P3.SP2 Five-Tier Test Execution Script (Linux/macOS)
# Accessibility, Localization & UI/UX Polish Verification Across Form Factors
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=== S8.P3.SP2 Five-Tier Verification Suite ==="

# 1. UNIT TESTS
echo "[1/5] Running [Unit] tests..."
cargo test -p ui-a11y --test ui_a11y_tests test_i18n_locale_key_parity_and_placeholder_variable_matching_all_6_locales
cargo test -p ui-a11y --test ui_a11y_tests test_wcag_relative_luminance_and_contrast_ratio_theme_palette_verifier
echo "  -> [Unit] PASS"

# 2. INTEGRATION TESTS
echo "[2/5] Running [Integration] tests..."
cargo test -p ui-a11y --test ui_a11y_tests test_svelte5_playwright_axe_core_wcag21aa_all_routes_zero_violations
cargo test -p ui-a11y --test ui_a11y_tests test_flutter_a11y_guidelines_contrast_tap_targets_and_semantics_all_screens
cargo test -p ui-a11y --test ui_a11y_tests test_6_form_factors_and_200_percent_text_scale_zero_overflow_all_locales
echo "  -> [Integration] PASS"

# 3. SIMULATION TESTS
echo "[3/5] Running [Simulation] tests..."
cargo test -p ui-a11y --test ui_a11y_tests test_keyboard_only_tab_traversal_and_modal_focus_trap_simulation
echo "  -> [Simulation] PASS"

# 4. HARDWARE-REQUIRED TESTS
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p ui-a11y --test ui_a11y_tests test_viewport_physical_resolution_and_touch_target_ergonomics -- --nocapture
echo "  -> [Hardware-Required] PASS"

# 5. MANUALLY-VERIFIED TESTS
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p ui-a11y --test ui_a11y_tests test_manual_rtl_arabic_and_german_compound_word_schema
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P3.SP2 All 5 Tiers PASSED (100% Pass Rate) ==="

