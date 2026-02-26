import en from '../../i18n/en.json';
import zhTW from '../../i18n/zh-TW.json';

const SUPPORTED = ['en', 'zh-TW'] as const;
const FALLBACK = 'en';

type Translations = Record<string, string>;

const locales: Record<string, Translations> = {
  en: en as Translations,
  'zh-TW': zhTW as Translations,
};

let currentLocale = $state(FALLBACK);

function resolve(translations: Translations, key: string): string | undefined {
  // Try flat key first
  if (translations[key] !== undefined) return translations[key];
  // Try nested path
  const parts = key.split('.');
  let obj: unknown = translations;
  for (const part of parts) {
    if (obj && typeof obj === 'object' && part in (obj as Record<string, unknown>)) {
      obj = (obj as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return typeof obj === 'string' ? obj : undefined;
}

export function t(key: string, params?: Record<string, string | number>): string {
  // Access currentLocale to create a reactive dependency
  const locale = currentLocale;
  let val = resolve(locales[locale], key);
  if (val === undefined && locale !== FALLBACK) {
    val = resolve(locales[FALLBACK], key);
  }
  if (val === undefined) return key;

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      val = val!.replaceAll(`{${k}}`, String(v));
    }
  }
  return val!;
}

export function getLocale(): string {
  return currentLocale;
}

export function setLocale(locale: string) {
  if (locales[locale]) {
    currentLocale = locale;
    localStorage.setItem('sumi-lang', locale);
  }
}

export function detectLocale(): string {
  const nav = navigator.language;
  if (nav.startsWith('zh')) return 'zh-TW';
  return 'en';
}

export function initLocale(savedLocale?: string | null) {
  const locale = savedLocale || localStorage.getItem('sumi-lang') || detectLocale();
  if (locales[locale]) {
    currentLocale = locale;
  }
}

export function getSupportedLocales() {
  return SUPPORTED;
}
