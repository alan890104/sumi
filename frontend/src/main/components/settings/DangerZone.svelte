<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { resetOnboarding, buildPayload } from '$lib/stores/settings.svelte';
  import { showConfirm, setShowSetup } from '$lib/stores/ui.svelte';
  import { resetSettings as apiResetSettings, saveSettings as saveSettingsApi } from '$lib/api';
  import { load as loadSettings } from '$lib/stores/settings.svelte';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';

  async function handleReset() {
    showConfirm(
      t('confirm.resetTitle'),
      t('confirm.resetMessage'),
      t('confirm.reset'),
      async () => {
        try {
          await apiResetSettings();
          await loadSettings();
        } catch (e) {
          console.error('Failed to reset settings:', e);
        }
      }
    );
  }

  async function handleRerunSetup() {
    resetOnboarding();
    try {
      await saveSettingsApi(buildPayload());
    } catch (e) {
      console.error('Failed to save onboarding reset:', e);
    }
    setShowSetup(true);
  }
</script>

<div class="section">
  <SectionHeader title={t('settings.danger')}>
    {#snippet icon()}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>
    {/snippet}
  </SectionHeader>

  <SettingRow name={t('settings.danger.reset')} desc={t('settings.danger.resetDesc')}>
    <button class="reset-btn" onclick={handleReset}>{t('settings.danger.resetBtn')}</button>
  </SettingRow>

  <SettingRow name={t('settings.danger.rerunSetup')} desc={t('settings.danger.rerunSetupDesc')}>
    <button class="reset-btn" onclick={handleRerunSetup}>{t('settings.danger.rerunSetupBtn')}</button>
  </SettingRow>
</div>

<style>
  .section {
    margin-bottom: 32px;
  }


  .reset-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 7px 16px;
    border: 1px solid rgba(255, 59, 48, 0.3);
    border-radius: var(--radius-sm);
    background: rgba(255, 59, 48, 0.08);
    color: #ff3b30;
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .reset-btn:hover {
    background: rgba(255, 59, 48, 0.15);
    border-color: rgba(255, 59, 48, 0.4);
  }
</style>
