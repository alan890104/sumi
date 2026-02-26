<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { initLocale } from '$lib/stores/i18n.svelte';
  import { getCurrentPage, setShowSetup } from '$lib/stores/ui.svelte';
  import * as settingsStore from '$lib/stores/settings.svelte';

  import Sidebar from './components/Sidebar.svelte';
  import ConfirmModal from './components/ConfirmModal.svelte';
  import SetupOverlay from './components/SetupOverlay.svelte';

  import SettingsPage from './pages/SettingsPage.svelte';
  import PromptRulesPage from './pages/PromptRulesPage.svelte';
  import DictionaryPage from './pages/DictionaryPage.svelte';
  import HistoryPage from './pages/HistoryPage.svelte';
  import AboutPage from './pages/AboutPage.svelte';
  import TestWizard from './pages/TestWizard.svelte';

  let version = $state('');

  onMount(async () => {
    // Get app version
    try {
      version = await getVersion();
    } catch {
      version = '0.0.0';
    }

    // Load settings & init i18n
    await settingsStore.load();
    const settings = settingsStore.getSettings();
    await initLocale(settings.language);

    // Check onboarding
    if (!settingsStore.getOnboardingCompleted()) {
      setShowSetup(true);
    }
  });
</script>

<div class="app">
  <Sidebar {version} />
  <div class="content-area">
    <div class="content-drag"></div>
    <div class="content-scroll">
      {#if getCurrentPage() === 'settings'}
        <SettingsPage />
      {:else if getCurrentPage() === 'promptRules'}
        <PromptRulesPage />
      {:else if getCurrentPage() === 'dictionary'}
        <DictionaryPage />
      {:else if getCurrentPage() === 'history'}
        <HistoryPage />
      {:else if getCurrentPage() === 'about'}
        <AboutPage />
      {:else if getCurrentPage() === 'test'}
        <TestWizard />
      {/if}
    </div>
  </div>

  <SetupOverlay />
  <ConfirmModal />
</div>

<style>
  .app {
    display: flex;
    width: 100%;
    height: 100vh;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content-drag {
    height: 52px;
    -webkit-app-region: drag;
    app-region: drag;
    flex-shrink: 0;
  }

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 36px 36px;
  }
</style>
