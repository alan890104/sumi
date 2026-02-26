<script lang="ts">
  let { options, value, onchange }: {
    options: { value: string; label: string }[];
    value: string;
    onchange: (value: string) => void;
  } = $props();

  function handleChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    onchange(target.value);
  }
</script>

<select class="select" value={value} onchange={handleChange}>
  {#each options as option}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>

<style>
  .select {
    padding: 7px 28px 7px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: 'Inter', -apple-system, sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    background: var(--bg-primary);
    cursor: pointer;
    outline: none;
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L5 5L9 1' stroke='%236e6e73' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    min-width: 160px;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .select:hover {
    border-color: rgba(0, 0, 0, 0.15);
    background-color: var(--bg-sidebar);
  }

  .select:focus {
    outline: none;
    border-color: var(--accent-blue);
  }
</style>
