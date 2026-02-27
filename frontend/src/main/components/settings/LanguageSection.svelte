<script lang="ts">
  import { t, getLocale, setLocale } from '$lib/stores/i18n.svelte';
  import { setLanguage, save } from '$lib/stores/settings.svelte';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';
  import Select from '$lib/components/Select.svelte';

  const languageOptions = [
    { value: 'en', label: 'English' },
    { value: 'af', label: 'Afrikaans' },
    { value: 'ar', label: 'العربية' },
    { value: 'hy', label: 'Հայերեն' },
    { value: 'az', label: 'Azərbaycan' },
    { value: 'be', label: 'Беларуская' },
    { value: 'bs', label: 'Bosanski' },
    { value: 'bg', label: 'Български' },
    { value: 'ca', label: 'Català' },
    { value: 'zh-CN', label: '简体中文' },
    { value: 'zh-TW', label: '繁體中文' },
    { value: 'hr', label: 'Hrvatski' },
    { value: 'cs', label: 'Čeština' },
    { value: 'da', label: 'Dansk' },
    { value: 'nl', label: 'Nederlands' },
    { value: 'et', label: 'Eesti' },
    { value: 'fi', label: 'Suomi' },
    { value: 'fr', label: 'Français' },
    { value: 'gl', label: 'Galego' },
    { value: 'de', label: 'Deutsch' },
    { value: 'el', label: 'Ελληνικά' },
    { value: 'he', label: 'עברית' },
    { value: 'hi', label: 'हिन्दी' },
    { value: 'hu', label: 'Magyar' },
    { value: 'is', label: 'Íslenska' },
    { value: 'id', label: 'Bahasa Indonesia' },
    { value: 'it', label: 'Italiano' },
    { value: 'ja', label: '日本語' },
    { value: 'kn', label: 'ಕನ್ನಡ' },
    { value: 'kk', label: 'Қазақ' },
    { value: 'ko', label: '한국어' },
    { value: 'lv', label: 'Latviešu' },
    { value: 'lt', label: 'Lietuvių' },
    { value: 'mk', label: 'Македонски' },
    { value: 'ms', label: 'Bahasa Melayu' },
    { value: 'mr', label: 'मराठी' },
    { value: 'mi', label: 'Te Reo Māori' },
    { value: 'ne', label: 'नेपाली' },
    { value: 'no', label: 'Norsk' },
    { value: 'fa', label: 'فارسی' },
    { value: 'pl', label: 'Polski' },
    { value: 'pt', label: 'Português' },
    { value: 'ro', label: 'Română' },
    { value: 'ru', label: 'Русский' },
    { value: 'sr', label: 'Српски' },
    { value: 'sk', label: 'Slovenčina' },
    { value: 'sl', label: 'Slovenščina' },
    { value: 'es', label: 'Español' },
    { value: 'sw', label: 'Kiswahili' },
    { value: 'sv', label: 'Svenska' },
    { value: 'tl', label: 'Tagalog' },
    { value: 'ta', label: 'தமிழ்' },
    { value: 'th', label: 'ไทย' },
    { value: 'tr', label: 'Türkçe' },
    { value: 'uk', label: 'Українська' },
    { value: 'ur', label: 'اردو' },
    { value: 'vi', label: 'Tiếng Việt' },
    { value: 'cy', label: 'Cymraeg' },
  ];

  function onLanguageChange(value: string) {
    setLocale(value);
    setLanguage(value);
    save();
  }
</script>

<div class="section">
  <SectionHeader title={t('settings.language')}>
    {#snippet icon()}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>
    {/snippet}
  </SectionHeader>
  <SettingRow name={t('settings.language.label')} desc={t('settings.language.desc')}>
    <Select
      options={languageOptions}
      value={getLocale()}
      onchange={onLanguageChange}
    />
  </SettingRow>
</div>

<style>
  .section {
    margin-bottom: 32px;
  }

</style>
