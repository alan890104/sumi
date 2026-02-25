/**
 * Lightweight i18n module for Voxink.
 *
 * Usage:
 *   <span data-i18n="nav.settings">Settings</span>
 *   <span data-i18n-html="setup.desc">...</span>
 *   <input data-i18n-placeholder="cloud.apiKeyPlaceholder" />
 *   t('overlay.recording')          // → "Recording"
 *   t('history.devices', { n: 3 })  // → "3 devices available"
 */

const I18n = (() => {
  const SUPPORTED = ['en', 'zh-TW'];
  const FALLBACK = 'en';

  let currentLocale = FALLBACK;
  let translations = {};   // { 'en': {...}, 'zh-TW': {...} }

  /** Resolve a dot-separated key from a nested or flat object. */
  function resolve(obj, key) {
    if (!obj) return undefined;
    // Try flat key first (our JSON is flat)
    if (obj[key] !== undefined) return obj[key];
    // Try nested path
    const parts = key.split('.');
    let cur = obj;
    for (const p of parts) {
      if (cur == null) return undefined;
      cur = cur[p];
    }
    return cur;
  }

  /**
   * Translate a key, with optional {param} interpolation.
   * Fallback order: current locale → English → key itself.
   */
  function t(key, params) {
    let val = resolve(translations[currentLocale], key);
    if (val === undefined && currentLocale !== FALLBACK) {
      val = resolve(translations[FALLBACK], key);
    }
    if (val === undefined) return key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        val = val.replace(new RegExp('\\{' + k + '\\}', 'g'), v);
      }
    }
    return val;
  }

  /** Scan the DOM and apply translations to data-i18n / data-i18n-html / data-i18n-placeholder elements. */
  function applyTranslations() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
      const key = el.getAttribute('data-i18n');
      el.textContent = t(key);
    });
    document.querySelectorAll('[data-i18n-html]').forEach(el => {
      const key = el.getAttribute('data-i18n-html');
      el.innerHTML = t(key);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
      const key = el.getAttribute('data-i18n-placeholder');
      el.placeholder = t(key);
    });
  }

  /**
   * Load a locale's JSON and switch to it.
   * @param {string} locale - e.g. 'en', 'zh-TW'
   */
  async function setLocale(locale) {
    if (!SUPPORTED.includes(locale)) locale = FALLBACK;
    if (!translations[locale]) {
      try {
        const resp = await fetch(`i18n/${locale}.json`);
        translations[locale] = await resp.json();
      } catch (e) {
        console.warn(`[i18n] Failed to load ${locale}:`, e);
        if (locale !== FALLBACK) {
          currentLocale = FALLBACK;
          applyTranslations();
          return;
        }
      }
    }
    currentLocale = locale;
    document.documentElement.lang = locale;
    applyTranslations();
  }

  /**
   * Detect locale from navigator.language.
   * zh-* → zh-TW, everything else → en.
   */
  function detectLocale() {
    const lang = (navigator.language || '').toLowerCase();
    if (lang.startsWith('zh')) return 'zh-TW';
    return 'en';
  }

  /**
   * Initialise i18n. Preload English (fallback), then switch to saved/detected locale.
   * @param {string|null} savedLocale - Previously saved locale, or null for auto-detect.
   */
  async function initI18n(savedLocale) {
    // Always preload fallback
    if (!translations[FALLBACK]) {
      try {
        const resp = await fetch(`i18n/${FALLBACK}.json`);
        translations[FALLBACK] = await resp.json();
      } catch (e) {
        console.warn('[i18n] Failed to load fallback locale:', e);
      }
    }
    const locale = savedLocale || detectLocale();
    await setLocale(locale);
  }

  /** Return the current locale string. */
  function getLocale() {
    return currentLocale;
  }

  return { t, applyTranslations, setLocale, detectLocale, initI18n, getLocale };
})();

// Convenient global alias
const t = I18n.t;
