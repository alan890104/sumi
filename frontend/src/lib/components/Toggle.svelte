<script lang="ts">
  let { checked, onchange }: {
    checked: boolean;
    onchange?: (checked: boolean) => void;
  } = $props();

  function handleChange(e: Event) {
    const target = e.target as HTMLInputElement;
    onchange?.(target.checked);
  }
</script>

<label class="toggle">
  <input type="checkbox" checked={checked} onchange={handleChange} />
  <div class="toggle-track"></div>
</label>

<style>
  .toggle {
    position: relative;
    width: 44px;
    height: 26px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
    position: absolute;
  }

  .toggle-track {
    position: absolute;
    inset: 0;
    border-radius: 13px;
    background: rgba(0, 0, 0, 0.12);
    transition: all 0.25s ease;
    cursor: pointer;
  }

  .toggle-track::after {
    content: '';
    position: absolute;
    width: 22px;
    height: 22px;
    background: #fff;
    border-radius: 50%;
    left: 2px;
    top: 2px;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }

  .toggle input:checked + .toggle-track {
    background: var(--accent-green);
  }

  .toggle input:checked + .toggle-track::after {
    transform: translateX(18px);
  }
</style>
