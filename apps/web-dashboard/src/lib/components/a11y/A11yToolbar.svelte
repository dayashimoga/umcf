<script lang="ts">
  import { i18n, SUPPORTED_LOCALES, type SupportedLocale, type AppTheme } from '../../i18n/i18n.svelte';

  let currentLocale = $derived(i18n.locale);
  let currentTheme = $derived(i18n.theme);
  let reducedMotion = $derived(i18n.reducedMotion);
  let textScale = $derived(i18n.textScale);

  function handleLocaleChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    i18n.setLocale(target.value as SupportedLocale);
  }

  function handleThemeChange(newTheme: AppTheme) {
    i18n.setTheme(newTheme);
  }

  function toggleReducedMotion() {
    i18n.setReducedMotion(!i18n.reducedMotion);
  }

  function cycleTextScale() {
    const scales = [100, 125, 150, 175, 200];
    const currentIndex = scales.indexOf(textScale);
    const nextIndex = (currentIndex + 1) % scales.length;
    i18n.setTextScale(scales[nextIndex] ?? 100);
  }
</script>

<div class="a11y-toolbar" role="region" aria-label="Accessibility and Internationalization Options">
  <!-- Skip Link for Keyboard Navigation (WCAG 2.4.1 Bypass Blocks) -->
  <a href="#main-content" class="skip-link">
    {i18n.t('skip_to_content')}
  </a>

  <!-- Accessibility Controls Cluster -->
  <div class="controls-cluster">
    <!-- Language Selector with Native Names -->
    <div class="control-group">
      <label for="locale-select" class="sr-only">{i18n.t('language_selector')}</label>
      <span class="toolbar-icon" aria-hidden="true">🌐</span>
      <select
        id="locale-select"
        class="a11y-select"
        value={currentLocale}
        onchange={handleLocaleChange}
        aria-label={i18n.t('language_selector')}
      >
        {#each SUPPORTED_LOCALES as loc}
          <option value={loc.code}>{loc.nativeName} ({loc.name})</option>
        {/each}
      </select>
    </div>

    <!-- Theme Switcher (Dark, Light, High-Contrast) -->
    <div class="control-group" role="group" aria-label="Color Themes">
      <button
        type="button"
        class="theme-btn {currentTheme === 'dark' ? 'active' : ''}"
        onclick={() => handleThemeChange('dark')}
        aria-pressed={currentTheme === 'dark'}
        title={i18n.t('theme_dark')}
      >
        🌙 <span class="btn-text">{i18n.t('theme_dark')}</span>
      </button>

      <button
        type="button"
        class="theme-btn {currentTheme === 'light' ? 'active' : ''}"
        onclick={() => handleThemeChange('light')}
        aria-pressed={currentTheme === 'light'}
        title={i18n.t('theme_light')}
      >
        ☀️ <span class="btn-text">{i18n.t('theme_light')}</span>
      </button>

      <button
        type="button"
        class="theme-btn high-contrast-btn {currentTheme === 'high-contrast' ? 'active' : ''}"
        onclick={() => handleThemeChange('high-contrast')}
        aria-pressed={currentTheme === 'high-contrast'}
        title={i18n.t('theme_high_contrast')}
      >
        ◐ <span class="btn-text">{i18n.t('theme_high_contrast')}</span>
      </button>
    </div>

    <!-- Reduced Motion Toggle -->
    <button
      type="button"
      class="toggle-btn {reducedMotion ? 'active' : ''}"
      onclick={toggleReducedMotion}
      aria-pressed={reducedMotion}
      title={i18n.t('reduced_motion')}
    >
      🎬 <span class="btn-text">{i18n.t('reduced_motion')}</span>
    </button>

    <!-- Dynamic Text Scaling (100% -> 200%) -->
    <button
      type="button"
      class="toggle-btn"
      onclick={cycleTextScale}
      aria-label={i18n.t('text_scale', { scale: textScale })}
      title={i18n.t('text_scale', { scale: textScale })}
    >
      🔤 <span class="btn-text">{textScale}%</span>
    </button>

    <!-- WCAG 2.1 AA Compliance Badge -->
    <div class="badge-wcag" title={i18n.t('wcag_compliant')} role="status">
      <span class="badge-icon" aria-hidden="true">♿</span>
      <span class="badge-label">WCAG 2.1 AA</span>
    </div>
  </div>
</div>

<style>
  .a11y-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background-color: var(--a11y-bar-bg, #0d1117);
    border-bottom: 1px solid var(--a11y-bar-border, #30363d);
    font-size: 0.875rem;
    position: relative;
    z-index: 100;
  }

  .skip-link {
    position: absolute;
    top: -40px;
    left: 10px;
    background: #238636;
    color: #ffffff;
    padding: 0.5rem 1rem;
    font-weight: bold;
    border-radius: 4px;
    z-index: 1000;
    transition: top 0.2s ease;
    text-decoration: none;
  }

  .skip-link:focus {
    top: 8px;
    outline: 3px solid #58a6ff;
  }

  .controls-cluster {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-inline-start: auto;
  }

  .control-group {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .toolbar-icon {
    font-size: 1rem;
    margin-inline-end: 0.25rem;
  }

  .a11y-select {
    background: #161b22;
    color: #e6edf3;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 0.35rem 0.5rem;
    font-size: 0.8125rem;
    cursor: pointer;
  }

  .a11y-select:focus-visible,
  .theme-btn:focus-visible,
  .toggle-btn:focus-visible {
    outline: 2px solid #58a6ff;
    outline-offset: 2px;
  }

  .theme-btn,
  .toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: #161b22;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 0.35rem 0.6rem;
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .theme-btn:hover,
  .toggle-btn:hover {
    background: #21262d;
    color: #c9d1d9;
  }

  .theme-btn.active,
  .toggle-btn.active {
    background: #1f6feb;
    color: #ffffff;
    border-color: #388bfd;
    font-weight: 600;
  }

  .high-contrast-btn.active {
    background: #ffff00;
    color: #000000;
    border-color: #ffffff;
    font-weight: bold;
  }

  .badge-wcag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: rgba(35, 134, 54, 0.2);
    border: 1px solid #2ea043;
    color: #3fb950;
    border-radius: 9999px;
    font-weight: 600;
    font-size: 0.75rem;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* Responsive compact view for mobile viewports */
  @media (max-width: 768px) {
    .btn-text {
      display: none;
    }
    .badge-wcag .badge-label {
      display: none;
    }
  }
</style>
