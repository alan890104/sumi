<script lang="ts">
  let { percent, label, sublabel, shimmer = false }: {
    percent: number;
    label?: string;
    sublabel?: string;
    shimmer?: boolean;
  } = $props();
</script>

<div class="progress-wrap">
  <div class="progress-track">
    <div
      class="progress-fill"
      class:shimmer
      style="width: {percent}%"
    ></div>
  </div>
  {#if label || sublabel}
    <div class="progress-label">
      <span>{label ?? ''}</span>
      <span>{sublabel ?? ''}</span>
    </div>
  {/if}
</div>

<style>
  .progress-wrap {
    margin-top: 10px;
  }

  .progress-track {
    width: 100%;
    height: 6px;
    background: rgba(0, 0, 0, 0.06);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    width: 0%;
    background: var(--accent-blue);
    border-radius: 3px;
    transition: width 0.15s ease;
    position: relative;
    overflow: hidden;
  }

  .progress-fill.shimmer::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 255, 255, 0.3) 50%,
      transparent 100%
    );
    animation: shimmer 1.5s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  .progress-label {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
