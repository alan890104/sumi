<script lang="ts">
  let { options, value, onchange }: {
    options: { value: string; label: string }[];
    value: string;
    onchange: (value: string) => void;
  } = $props();
</script>

<div class="segmented-control">
  {#each options as option}
    <button
      class:active={value === option.value}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented-control {
    display: flex;
    background: rgba(0, 0, 0, 0.06);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }

  .segmented-control button {
    flex: 1;
    padding: 6px 16px;
    border: none;
    border-radius: 6px;
    background: transparent;
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
  }

  .segmented-control button.active {
    background: #fff;
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  .segmented-control button:hover:not(.active) {
    color: var(--text-primary);
  }
</style>
