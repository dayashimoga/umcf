import enUs from './locales/en-US.json';
import esEs from './locales/es-ES.json';
import deDe from './locales/de-DE.json';
import jaJp from './locales/ja-JP.json';
import zhCn from './locales/zh-CN.json';
import arSa from './locales/ar-SA.json';

export type SupportedLocale = 'en-US' | 'es-ES' | 'de-DE' | 'ja-JP' | 'zh-CN' | 'ar-SA';
export type AppTheme = 'dark' | 'light' | 'high-contrast';

export interface LocaleInfo {
  code: SupportedLocale;
  name: string;
  nativeName: string;
  isRtl: boolean;
}

export const SUPPORTED_LOCALES: LocaleInfo[] = [
  { code: 'en-US', name: 'English (US)', nativeName: 'English', isRtl: false },
  { code: 'es-ES', name: 'Spanish', nativeName: 'Español', isRtl: false },
  { code: 'de-DE', name: 'German', nativeName: 'Deutsch', isRtl: false },
  { code: 'ja-JP', name: 'Japanese', nativeName: '日本語', isRtl: false },
  { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '简体中文', isRtl: false },
  { code: 'ar-SA', name: 'Arabic (Saudi Arabia)', nativeName: 'العربية', isRtl: true },
];

const CATALOGS: Record<SupportedLocale, Record<string, string>> = {
  'en-US': enUs,
  'es-ES': esEs,
  'de-DE': deDe,
  'ja-JP': jaJp,
  'zh-CN': zhCn,
  'ar-SA': arSa,
};

class I18nManager {
  locale = $state<SupportedLocale>('en-US');
  theme = $state<AppTheme>('dark');
  reducedMotion = $state<boolean>(false);
  textScale = $state<number>(100);

  isRtl = $derived(this.locale === 'ar-SA');

  constructor() {
    if (typeof window !== 'undefined') {
      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      this.reducedMotion = prefersReducedMotion;
      this.applyDomAttributes();
    }
  }

  setLocale(newLocale: SupportedLocale) {
    this.locale = newLocale;
    this.applyDomAttributes();
  }

  setTheme(newTheme: AppTheme) {
    this.theme = newTheme;
    this.applyDomAttributes();
  }

  setReducedMotion(enabled: boolean) {
    this.reducedMotion = enabled;
    this.applyDomAttributes();
  }

  setTextScale(percent: number) {
    this.textScale = Math.max(100, Math.min(200, percent));
    this.applyDomAttributes();
  }

  private applyDomAttributes() {
    if (typeof document !== 'undefined') {
      document.documentElement.dir = this.isRtl ? 'rtl' : 'ltr';
      document.documentElement.lang = this.locale;
      document.documentElement.dataset.theme = this.theme;
      document.documentElement.dataset.reducedMotion = this.reducedMotion ? 'true' : 'false';
      document.documentElement.style.fontSize = `${(this.textScale / 100) * 16}px`;
    }
  }

  t(key: string, params?: Record<string, string | number>): string {
    const catalog = CATALOGS[this.locale] || CATALOGS['en-US'];
    let str = catalog[key] || CATALOGS['en-US'][key] || key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
      }
    }
    return str;
  }
}

export const i18n = new I18nManager();
