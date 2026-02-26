<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import LanguageSection from '../components/settings/LanguageSection.svelte';
  import HotkeySection from '../components/settings/HotkeySection.svelte';
  import MicSection from '../components/settings/MicSection.svelte';
  import SttSection from '../components/settings/SttSection.svelte';
  import PolishSection from '../components/settings/PolishSection.svelte';
  import DangerZone from '../components/settings/DangerZone.svelte';
</script>

<div class="page">
  <h1 class="page-title">{t('settings.title')}</h1>

  <LanguageSection />
  <div class="section-divider"></div>

  <HotkeySection />
  <div class="section-divider"></div>

  <MicSection />
  <div class="section-divider"></div>

  <SttSection />
  <div class="section-divider"></div>

  <PolishSection />
  <div class="section-divider"></div>

  <DangerZone />
</div>

<style>
  .page-title {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .section-divider {
    height: 1px;
    background: var(--border-divider);
    margin-bottom: 28px;
  }
</style>
