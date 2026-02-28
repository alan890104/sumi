import en from '../../i18n/en.json';

const SUPPORTED = [
  'en', 'zh-TW', 'zh-CN',
  'af', 'ar', 'hy', 'az', 'be', 'bs', 'bg', 'ca',
  'hr', 'cs', 'da', 'nl', 'et', 'fi', 'fr', 'gl',
  'de', 'el', 'he', 'hi', 'hu', 'is', 'id', 'it',
  'ja', 'kn', 'kk', 'ko', 'lv', 'lt', 'mk', 'ms',
  'mr', 'mi', 'ne', 'no', 'fa', 'pl', 'pt', 'ro',
  'ru', 'sr', 'sk', 'sl', 'es', 'sw', 'sv', 'tl',
  'ta', 'th', 'tr', 'uk', 'ur', 'vi', 'cy',
] as const;

const FALLBACK = 'en';

type Translations = Record<string, string>;

const locales: Record<string, Translations> = {
  en: en as Translations,
};

let currentLocale = $state(FALLBACK);

function resolve(translations: Translations, key: string): string | undefined {
  return translations[key];
}

export function t(key: string, params?: Record<string, string | number>): string {
  // Access currentLocale to create a reactive dependency
  const locale = currentLocale;
  let val = resolve(locales[locale] ?? locales[FALLBACK], key);
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

async function loadLocale(locale: string): Promise<Translations | null> {
  if (locales[locale]) return locales[locale];
  try {
    const mod = await import(`../../i18n/${locale}.json`);
    locales[locale] = mod.default as Translations;
    return locales[locale];
  } catch {
    return null;
  }
}

export async function setLocale(locale: string) {
  const translations = await loadLocale(locale);
  if (translations) {
    currentLocale = locale;
  }
}

export function detectLocale(): string {
  const nav = navigator.language;
  // Exact match first
  if ((SUPPORTED as readonly string[]).includes(nav)) return nav;
  // zh variants
  if (nav.startsWith('zh')) {
    if (nav.includes('CN') || nav.includes('Hans') || nav === 'zh-SG') return 'zh-CN';
    return 'zh-TW';
  }
  // Match by primary language subtag
  const primary = nav.split('-')[0];
  if ((SUPPORTED as readonly string[]).includes(primary)) return primary;
  return 'en';
}

export async function initLocale(savedLocale?: string | null) {
  const locale = savedLocale || detectLocale();
  if ((SUPPORTED as readonly string[]).includes(locale)) {
    await loadLocale(locale);
    currentLocale = locale;
  }
}

export function getSupportedLocales() {
  return SUPPORTED;
}
