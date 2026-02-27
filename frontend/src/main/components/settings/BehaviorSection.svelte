<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { getSettings, setAutoPaste, save } from '$lib/stores/settings.svelte';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';
  import Toggle from '$lib/components/Toggle.svelte';

  const settings = $derived(getSettings());

  function onToggleAutoPaste(checked: boolean) {
    setAutoPaste(checked);
    save();
  }
</script>

<div class="section">
  <SectionHeader title={t('settings.behavior')}>
    {#snippet icon()}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
        <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
        <path d="m9 14 2 2 4-4"/>
      </svg>
    {/snippet}
  </SectionHeader>

  <SettingRow name={t('settings.behavior.autoPaste')} desc={t('settings.behavior.autoPasteDesc')}>
    <Toggle checked={settings.auto_paste} onchange={onToggleAutoPaste} />
  </SettingRow>
</div>

<style>
  .section {
    margin-bottom: 32px;
  }

</style>
