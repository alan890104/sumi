<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { getCurrentPage, setCurrentPage } from '$lib/stores/ui.svelte';
  import { isDevMode } from '$lib/api';
  import type { Page } from '$lib/types';

  let { version = '' }: { version?: string } = $props();
  let devMode = $state(false);
  isDevMode().then((v) => (devMode = v));

  const navItems: { id: Page; labelKey: string; icon: string }[] = [
    {
      id: 'stats',
      labelKey: 'nav.stats',
      icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>',
    },
    { id: 'settings', labelKey: 'nav.settings', icon: '⚙' },
    {
      id: 'promptRules',
      labelKey: 'nav.promptRules',
      icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>',
    },
    {
      id: 'dictionary',
      labelKey: 'nav.dictionary',
      icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>',
    },
    {
      id: 'history',
      labelKey: 'nav.history',
      icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
    },
    {
      id: 'test',
      labelKey: 'nav.test',
      icon: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>',
    },
    { id: 'about', labelKey: 'nav.about', icon: 'ⓘ' },
  ];

  function handleClick(id: Page) {
    setCurrentPage(id);
  }
</script>

<div class="sidebar">
  <div class="sidebar-drag">
    <div class="sidebar-brand">
      <img src="/icon.png" alt="Sumi" class="sidebar-logo" />
      <span class="sidebar-app-name">Sumi</span>
      {#if devMode}
        <span class="dev-badge">Dev</span>
      {/if}
    </div>
  </div>
  <nav class="sidebar-nav">
    {#each navItems as item}
      <button
        class="nav-item"
        class:active={getCurrentPage() === item.id}
        onclick={() => handleClick(item.id)}
      >
        <span class="nav-icon">{@html item.icon}</span>
        <span>{t(item.labelKey)}</span>
      </button>
    {/each}
  </nav>
  <div class="sidebar-footer">
    <div class="sidebar-version">{version}</div>
  </div>
</div>

<style>
  .sidebar {
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .sidebar-drag {
    height: 56px;
    padding-top: 12px;
    padding-bottom: 16px;
    -webkit-app-region: drag;
    app-region: drag;
    flex-shrink: 0;
    display: flex;
    align-items: flex-end;
    box-sizing: content-box;
  }

  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 20px;
  }

  .sidebar-logo {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .sidebar-app-name {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.01em;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .dev-badge {
    font-size: 10px;
    font-weight: 600;
    color: #fff;
    background: #f59e0b;
    padding: 1px 6px;
    border-radius: 4px;
    letter-spacing: 0.02em;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .sidebar-nav {
    flex: 1;
    padding: 0 12px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .nav-item {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
    border: none;
    background: none;
    width: 100%;
    text-align: left;
    font-family: inherit;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--bg-active);
    color: var(--text-primary);
    font-weight: 600;
  }

  .nav-icon {
    font-size: 17px;
    width: 22px;
    text-align: center;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sidebar-footer {
    padding: 16px 24px;
    border-top: 1px solid var(--border-divider);
  }

  .sidebar-version {
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
