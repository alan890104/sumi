<script lang="ts">
  import { t, getLocale, setLocale } from '$lib/stores/i18n.svelte';
  import { setLanguage, save } from '$lib/stores/settings.svelte';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import Select from '$lib/components/Select.svelte';

  const languageOptions = [
    { value: 'en', label: 'English' },
    { value: 'zh-TW', label: '\u7E41\u9AD4\u4E2D\u6587' },
  ];

  function onLanguageChange(value: string) {
    setLocale(value);
    setLanguage(value);
    save();
  }
</script>

<div class="section">
  <div class="section-header">
    <span class="section-icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>
    </span>
    <span class="section-title">{t('settings.language')}</span>
  </div>
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
    margin-bottom: 28px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 16px;
  }

  .section-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .section-icon :global(svg) {
    width: 18px;
    height: 18px;
    display: block;
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--text-secondary);
  }
</style>
