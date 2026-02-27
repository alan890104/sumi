<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { t, initLocale } from '$lib/stores/i18n.svelte';
  import {
    onRecordingStatus,
    onRecordingMaxDuration,
    onAudioLevels,
    onPreviewText,
    triggerUndo,
    confirmPreview,
    cancelPreview,
  } from '$lib/api';
  import { formatHotkeyDisplay, hotkeyToParts } from '$lib/constants';
  import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
  import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import type { OverlayStatus } from '$lib/types';

  // ── Constants ──
  const NUM_BARS = 20;
  const BAR_W = 2;
  const BAR_GAP = 2;
  const CW = NUM_BARS * (BAR_W + BAR_GAP) - BAR_GAP; // 78
  const CH = 24;
  const UNDO_DURATION = 5000;
  const TIMER_INTERVAL = 200;
  const INTERPOLATION_FACTOR = 0.25;
  const PREVIEW_WIDTH = 420;
  const CAPSULE_WIDTH = 300;
  const CAPSULE_HEIGHT = 52;

  // ── State ──
  type Phase =
    | 'preparing'
    | 'recording'
    | 'processing'
    | 'transcribing'
    | 'polishing'
    | 'pasted'
    | 'copied'
    | 'error'
    | 'edited'
    | 'edit_requires_polish'
    | 'undo'
    | 'preview';

  let phase: Phase = $state('preparing');
  let timerText: string = $state('0:00');
  let recProgress: number = $state(0);
  let maxDuration: number = $state(30);
  let undoAnimating: boolean = $state(false);

  // ── Preview state ──
  let previewText: string = $state('');
  let previewHotkey: string = $state('');
  let hotkeyParts: string[] = $derived(previewHotkey ? hotkeyToParts(previewHotkey) : []);
  let isEditing: boolean = $state(false);
  let editText: string = $state('');
  let textareaEl: HTMLTextAreaElement | undefined = $state();

  // ── Canvas & waveform ──
  let canvasEl: HTMLCanvasElement | undefined = $state();
  let waveCtx: CanvasRenderingContext2D | null = null;
  let currentLevels = new Array(NUM_BARS).fill(0);
  let targetLevels = new Array(NUM_BARS).fill(0);
  let waveAnimId: number | null = null;

  // ── Timers ──
  let startTime = 0;
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let undoTimeout: ReturnType<typeof setTimeout> | null = null;
  let editedTimeout: ReturnType<typeof setTimeout> | null = null;

  // ── Undo bar element for reflow trick ──
  let undoBarEl: HTMLDivElement | undefined = $state();

  // ── Event unlisteners ──
  let unlisteners: UnlistenFn[] = [];

  // ── Capsule class computation ──
  let capsuleClass: string = $derived.by(() => {
    switch (phase) {
      case 'preparing':
        return 'capsule preparing';
      case 'recording':
        return 'capsule recording';
      case 'processing':
        return 'capsule processing';
      case 'transcribing':
        return 'capsule transcribing';
      case 'polishing':
        return 'capsule polishing';
      case 'pasted':
      case 'copied':
      case 'edited':
        return 'capsule result success';
      case 'error':
      case 'edit_requires_polish':
        return 'capsule result error-state';
      case 'undo':
        return 'capsule undo-state';
      case 'preview':
        return 'capsule preview-state';
      default:
        return 'capsule';
    }
  });

  // ── Label text computation ──
  let labelText: string = $derived.by(() => {
    switch (phase) {
      case 'preparing':
        return t('overlay.preparing');
      case 'recording':
        return t('overlay.recording');
      case 'processing':
      case 'transcribing':
        return t('overlay.transcribing');
      case 'polishing':
        return t('overlay.polishing');
      case 'pasted':
        return t('overlay.pasted');
      case 'copied':
        return t('overlay.copied');
      case 'error':
        return t('overlay.failed');
      case 'edit_requires_polish':
        return t('overlay.editRequiresPolish');
      case 'edited':
        return t('overlay.edited');
      case 'undo':
        return t('overlay.undo');
      case 'preview':
        return '';
      default:
        return '';
    }
  });

  // ── Icon state derivations ──
  // Use helper to avoid TS narrowing issues with union types in $derived
  function is(...phases: Phase[]): boolean {
    return phases.includes(phase);
  }

  let showDot: boolean = $derived.by(() => false); // dot is never shown in practice (CSS handles it on .recording)
  let showSpinner: boolean = $derived.by(() => is('preparing', 'processing', 'transcribing', 'polishing'));
  let showWaveform: boolean = $derived.by(() => is('recording'));
  let showIconResult: boolean = $derived.by(() => is('pasted', 'copied', 'error', 'edit_requires_polish', 'edited'));
  let showTimer: boolean = $derived.by(() => is('recording'));
  let showUndoIcon: boolean = $derived.by(() => is('undo'));
  let showUndoBar: boolean = $derived.by(() => is('undo'));
  let isCheckIcon: boolean = $derived.by(() => is('pasted', 'copied', 'edited'));
  let isErrorIcon: boolean = $derived.by(() => is('error', 'edit_requires_polish'));
  let isPolishSpinner: boolean = $derived.by(() => is('polishing'));
  let showPreview: boolean = $derived.by(() => is('preview'));
  let showCapsuleContent: boolean = $derived.by(() => !is('preview'));

  // ── Waveform animation ──
  function animateWaveform() {
    if (!waveCtx) return;
    waveCtx.clearRect(0, 0, CW, CH);
    waveCtx.fillStyle = 'rgba(255, 255, 255, 0.85)';
    for (let i = 0; i < NUM_BARS; i++) {
      currentLevels[i] += (targetLevels[i] - currentLevels[i]) * INTERPOLATION_FACTOR;
      const h = Math.max(3, currentLevels[i] * CH);
      const x = i * (BAR_W + BAR_GAP);
      const y = (CH - h) / 2;
      waveCtx.fillRect(x, y, BAR_W, h);
    }
    waveAnimId = requestAnimationFrame(animateWaveform);
  }

  function startWaveform() {
    currentLevels.fill(0);
    targetLevels.fill(0);
    if (!waveAnimId) animateWaveform();
  }

  function stopWaveform() {
    if (waveAnimId) {
      cancelAnimationFrame(waveAnimId);
      waveAnimId = null;
    }
    // Reset context so it gets re-initialized when canvas is re-rendered
    waveCtx = null;
  }

  // ── Timer ──
  function startTimer() {
    startTime = Date.now();
    timerText = '0:00';
    recProgress = 0;
    timerInterval = setInterval(() => {
      const elapsed = Math.floor((Date.now() - startTime) / 1000);
      const mins = Math.floor(elapsed / 60);
      const secs = elapsed % 60;
      timerText = `${mins}:${secs.toString().padStart(2, '0')}`;
      recProgress = Math.min(elapsed / maxDuration, 1);
    }, TIMER_INTERVAL);
  }

  function stopTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  // ── Undo ──
  function clearUndoTimeout() {
    if (undoTimeout) {
      clearTimeout(undoTimeout);
      undoTimeout = null;
    }
  }

  function clearEditedTimeout() {
    if (editedTimeout) {
      clearTimeout(editedTimeout);
      editedTimeout = null;
    }
  }

  // ── State transitions ──
  function clearCommon() {
    clearUndoTimeout();
    clearEditedTimeout();
    stopTimer();
    stopWaveform();
    undoAnimating = false;
  }

  function setPreparing() {
    clearCommon();
    isEditing = false;
    previewText = '';
    phase = 'preparing';
  }

  function setRecording() {
    clearCommon();
    isEditing = false;
    phase = 'recording';
    startTimer();
    startWaveform();
  }

  function setProcessing() {
    clearCommon();
    phase = 'processing';
  }

  function setTranscribing() {
    clearCommon();
    phase = 'transcribing';
  }

  function setPolishing() {
    clearCommon();
    phase = 'polishing';
  }

  function setPasted() {
    clearCommon();
    phase = 'pasted';
  }

  function setCopied() {
    clearCommon();
    phase = 'copied';
  }

  function setError() {
    clearCommon();
    phase = 'error';
  }

  function setEditRequiresPolish() {
    clearCommon();
    phase = 'edit_requires_polish';
  }

  function setEdited() {
    clearCommon();
    phase = 'edited';
    // After 500ms, transition to undo state
    editedTimeout = setTimeout(() => setUndo(), 500);
  }

  function setUndo() {
    phase = 'undo';

    // Trigger undo bar animation after a tick so the element is rendered
    requestAnimationFrame(() => {
      if (undoBarEl) {
        undoBarEl.style.animation = 'none';
        // Force reflow
        void undoBarEl.offsetWidth;
        undoBarEl.style.animation = `undoCountdown ${UNDO_DURATION}ms linear forwards`;
      }
      undoAnimating = true;
    });

    // Auto-clear after 5s
    undoTimeout = setTimeout(() => {
      undoAnimating = false;
    }, UNDO_DURATION);
  }

  async function setPreview(text: string, hotkey: string) {
    clearCommon();
    previewText = text;
    previewHotkey = hotkey;
    editText = text;
    isEditing = false;
    phase = 'preview';
    await resizeForPreview(text);
  }

  // ── Preview helpers ──
  async function resizeForPreview(text: string) {
    const win = getCurrentWindow();
    const charsPerLine = 38;
    const lines = Math.max(Math.ceil(text.length / charsPerLine), text.split('\n').length, 2);
    const textH = Math.min(lines * 22 + 16, 220);
    // text area + action bar (44px) + padding (28px top/bottom)
    const totalH = textH + 44 + 28;
    const clampedH = Math.min(Math.max(totalH, 130), 340);

    await win.setSize(new LogicalSize(PREVIEW_WIDTH, clampedH));
    const monitor = await currentMonitor();
    if (monitor) {
      const sw = monitor.size.width / monitor.scaleFactor;
      const sh = monitor.size.height / monitor.scaleFactor;
      await win.setPosition(new LogicalPosition(
        (sw - PREVIEW_WIDTH) / 2,
        sh - clampedH - 80
      ));
    }
  }

  async function resetOverlaySize() {
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(CAPSULE_WIDTH, CAPSULE_HEIGHT));
  }

  async function handlePreviewConfirm() {
    const text = isEditing ? editText : undefined;
    await resetOverlaySize();
    try {
      await confirmPreview(text);
    } catch (e) {
      console.error('Preview confirm failed:', e);
    }
  }

  async function handlePreviewCancel() {
    await resetOverlaySize();
    try {
      await cancelPreview();
    } catch (e) {
      console.error('Preview cancel failed:', e);
    }
  }

  async function handlePreviewEdit() {
    isEditing = true;
    editText = previewText;
    await tick();
    textareaEl?.focus();
  }

  function handlePreviewKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.metaKey) {
      e.preventDefault();
      handlePreviewConfirm();
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      handlePreviewCancel();
    }
  }

  // ── Undo ──
  async function handleUndoClick() {
    if (phase !== 'undo') return;
    clearUndoTimeout();
    undoAnimating = false;
    phase = 'preparing'; // Reset visual while undo processes

    try {
      await triggerUndo();
    } catch (e) {
      console.error('Undo failed:', e);
    }
  }

  function handleCapsuleClick() {
    if (phase === 'undo') {
      handleUndoClick();
    }
  }

  // ── Handle recording-status event ──
  function handleStatus(status: string) {
    switch (status as OverlayStatus) {
      case 'preparing':
        setPreparing();
        break;
      case 'recording':
        setRecording();
        break;
      case 'processing':
        setProcessing();
        break;
      case 'transcribing':
        setTranscribing();
        break;
      case 'polishing':
        setPolishing();
        break;
      case 'pasted':
        setPasted();
        break;
      case 'copied':
        setCopied();
        break;
      case 'error':
        setError();
        break;
      case 'edited':
        setEdited();
        break;
      case 'edit_requires_polish':
        setEditRequiresPolish();
        break;
      case 'preview':
        // Text will arrive via preview-text event
        break;
    }
  }

  // ── Canvas setup (runs when canvasEl is bound) ──
  $effect(() => {
    if (canvasEl && !waveCtx) {
      const dpr = window.devicePixelRatio || 1;
      canvasEl.width = CW * dpr;
      canvasEl.height = CH * dpr;
      canvasEl.style.width = CW + 'px';
      canvasEl.style.height = CH + 'px';
      const ctx = canvasEl.getContext('2d');
      if (ctx) {
        ctx.scale(dpr, dpr);
        waveCtx = ctx;
        // Canvas just became available — start animation if we're already recording
        if (phase === 'recording' && !waveAnimId) {
          animateWaveform();
        }
      }
    }
  });

  // ── Window visibility ──
  // When the backend hides the overlay (e.g. after "no speech detected"),
  // no status event is emitted. Reset state so the capsule doesn't show
  // stale text (like "transcribing") when next shown.
  async function handleVisibility() {
    if (document.hidden) {
      if (phase === 'preview') {
        await resetOverlaySize();
        try { await cancelPreview(); } catch {}
      }
      setPreparing();
    }
  }

  // ── Lifecycle ──
  onMount(async () => {
    // Init i18n from localStorage
    await initLocale(localStorage.getItem('sumi-lang'));

    // Set initial state
    setPreparing();

    // Reset state when window is hidden by backend
    document.addEventListener('visibilitychange', handleVisibility);

    // Listen for Tauri events
    const u1 = await onRecordingStatus(handleStatus);
    const u2 = await onRecordingMaxDuration((secs) => {
      maxDuration = secs;
    });
    const u3 = await onAudioLevels((levels) => {
      targetLevels = levels;
    });
    const u4 = await onPreviewText(({ text, hotkey }) => {
      setPreview(text, hotkey);
    });
    unlisteners = [u1, u2, u3, u4];
  });

  onDestroy(() => {
    document.removeEventListener('visibilitychange', handleVisibility);
    stopTimer();
    stopWaveform();
    clearUndoTimeout();
    clearEditedTimeout();
    for (const unlisten of unlisteners) {
      unlisten();
    }
  });
</script>

{#if showPreview}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="preview-container" onkeydown={handlePreviewKeydown}>
    {#if isEditing}
      <textarea
        class="preview-textarea"
        bind:value={editText}
        bind:this={textareaEl}
        onkeydown={handlePreviewKeydown}
      ></textarea>
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="preview-text" onclick={handlePreviewEdit}>
        {previewText}
      </div>
    {/if}
    <div class="preview-actions">
      {#if isEditing}
        <span class="preview-hint editing-hint">{t('overlay.previewEditingHint')}</span>
      {:else}
        <span class="preview-hint">{t('overlay.previewEditHint')}</span>
      {/if}
      <div class="preview-btns">
        <button class="preview-btn cancel-btn" onclick={handlePreviewCancel}>
          {t('overlay.previewCancel')}
        </button>
        <button class="preview-btn confirm-btn" onclick={handlePreviewConfirm}>
          {#if hotkeyParts.length > 0}
            <span class="keycap-group">
              {#each hotkeyParts as part}
                <kbd class="keycap">{part}</kbd>
              {/each}
            </span>
          {/if}
          <span>{t('overlay.previewConfirm')}</span>
        </button>
      </div>
    </div>
  </div>
{:else}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={capsuleClass}
    style:--rec-progress={recProgress}
    onclick={handleCapsuleClick}
  >
    <!-- Recording dot (only visible in default/idle states via CSS) -->
    {#if showDot}
      <div class="dot"></div>
    {/if}

    <!-- Spinner -->
    {#if showSpinner}
      <div class="spinner" class:polish-spinner={isPolishSpinner}></div>
    {/if}

    <!-- Waveform canvas -->
    {#if showWaveform}
      <canvas
        class="waveform"
        bind:this={canvasEl}
        width={CW}
        height={CH}
      ></canvas>
    {/if}

    <!-- Result icons -->
    {#if showIconResult}
      <div class="icon-result">
        {#if isCheckIcon}
          <div class="icon-check"></div>
        {:else if isErrorIcon}
          <div class="icon-error"></div>
        {/if}
      </div>
    {/if}

    <!-- Undo icon -->
    {#if showUndoIcon}
      <svg class="icon-undo" viewBox="0 0 24 24" fill="none" stroke="rgba(255, 149, 0, 0.9)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="1 4 1 10 7 10"></polyline>
        <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
      </svg>
    {/if}

    <!-- Label -->
    <span class="label">{labelText}</span>

    <!-- Timer -->
    {#if showTimer}
      <span class="timer">{timerText}</span>
    {/if}

    <!-- Undo countdown bar -->
    {#if showUndoBar}
      <div class="undo-bar" bind:this={undoBarEl}></div>
    {/if}
  </div>
{/if}

<style>
  :global(*),
  :global(*::before),
  :global(*::after) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
    height: 100%;
    -webkit-user-select: none;
    user-select: none;
  }

  :global(body) {
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  }

  /* ── Capsule ── */
  .capsule {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 22px;
    height: 40px;
    border-radius: 100px;
    background: rgba(12, 12, 16, 0.88);
    backdrop-filter: blur(40px) saturate(1.6);
    -webkit-backdrop-filter: blur(40px) saturate(1.6);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.45),
      0 0 0 0.5px rgba(255, 255, 255, 0.06),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.92);
    transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
    animation: fadeIn 0.25s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  /* ── Recording dot ── */
  .dot {
    position: relative;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #ff3b30;
    flex-shrink: 0;
    animation: dotPulse 1.8s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  .dot::after {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    background: rgba(255, 59, 48, 0.25);
    animation: ringPulse 1.8s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes dotPulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }

  @keyframes ringPulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.4;
    }
    50% {
      transform: scale(1.5);
      opacity: 0;
    }
  }

  /* ── Spinner ── */
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.12);
    border-top-color: #a78bfa;
    border-radius: 50%;
    flex-shrink: 0;
    animation: spin 0.7s linear infinite;
  }

  .spinner.polish-spinner {
    border-top-color: #c084fc;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ── Result icon ── */
  .icon-result {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: flex;
    position: relative;
  }

  /* Checkmark */
  .icon-check {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #34c759;
    position: relative;
  }

  .icon-check::after {
    content: '';
    position: absolute;
    top: 3.5px;
    left: 5.5px;
    width: 4px;
    height: 7px;
    border: solid #fff;
    border-width: 0 1.8px 1.8px 0;
    transform: rotate(45deg);
  }

  /* Error X */
  .icon-error {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ff3b30;
    position: relative;
  }

  .icon-error::before,
  .icon-error::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 8px;
    height: 1.8px;
    background: #fff;
    border-radius: 1px;
  }

  .icon-error::before {
    transform: translate(-50%, -50%) rotate(45deg);
  }

  .icon-error::after {
    transform: translate(-50%, -50%) rotate(-45deg);
  }

  /* ── Undo icon ── */
  .icon-undo {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  /* ── Undo countdown bar ── */
  .undo-bar {
    position: absolute;
    bottom: 0;
    left: 16px;
    right: 16px;
    height: 2px;
    border-radius: 1px;
    background: rgba(255, 149, 0, 0.5);
    transform-origin: left;
  }

  @keyframes undoCountdown {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }

  /* ── Text ── */
  .label {
    font-size: 13px;
    font-weight: 500;
    letter-spacing: -0.01em;
    white-space: nowrap;
    line-height: 1;
  }

  .timer {
    font-size: 12px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.4);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    margin-left: auto;
    line-height: 1;
  }

  /* ── Waveform canvas ── */
  .waveform {
    flex-shrink: 0;
    width: 78px;
    height: 24px;
  }

  /* ── State: Recording (orange -> red gradient via --rec-progress) ── */
  .capsule.recording {
    --rec-progress: 0;
    background: color-mix(
      in srgb,
      rgba(255, 140, 0, 0.92) calc((1 - var(--rec-progress)) * 100%),
      rgba(255, 59, 48, 0.92)
    );
    border-color: rgba(255, 200, 100, 0.15);
    box-shadow:
      0 8px 32px
        color-mix(
          in srgb,
          rgba(255, 140, 0, 0.35) calc((1 - var(--rec-progress)) * 100%),
          rgba(255, 59, 48, 0.35)
        ),
      0 0 0 0.5px rgba(255, 255, 255, 0.08),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.12);
  }

  /* ── State: Polishing (purple tint) ── */
  .capsule.polishing {
    border-color: rgba(167, 139, 250, 0.25);
    box-shadow:
      0 8px 32px rgba(167, 139, 250, 0.15),
      0 0 0 0.5px rgba(255, 255, 255, 0.06),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.06);
  }

  /* ── State: Result ── */
  .capsule.result.success {
    border-color: rgba(52, 199, 89, 0.2);
  }

  .capsule.result.error-state {
    border-color: rgba(255, 59, 48, 0.2);
  }

  /* ── State: Undo ── */
  .capsule.undo-state {
    position: relative;
    cursor: pointer;
    border-color: rgba(255, 149, 0, 0.3);
    box-shadow:
      0 8px 32px rgba(255, 149, 0, 0.15),
      0 0 0 0.5px rgba(255, 255, 255, 0.06),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.06);
    transition: all 0.15s ease;
  }

  .capsule.undo-state:hover {
    background: rgba(30, 30, 36, 0.92);
    border-color: rgba(255, 149, 0, 0.5);
  }

  .capsule.undo-state:active {
    transform: scale(0.97);
  }

  /* ── Preview container ── */
  .preview-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    padding: 14px 16px;
    background: rgba(12, 12, 16, 0.92);
    backdrop-filter: blur(40px) saturate(1.6);
    -webkit-backdrop-filter: blur(40px) saturate(1.6);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.45),
      0 0 0 0.5px rgba(255, 255, 255, 0.06),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.92);
    animation: fadeIn 0.25s ease-out;
  }

  .preview-text {
    flex: 1;
    font-size: 13.5px;
    line-height: 1.65;
    color: rgba(255, 255, 255, 0.88);
    overflow-y: auto;
    cursor: text;
    word-break: break-word;
    padding: 2px 0;
    border-radius: 6px;
    transition: background 0.15s ease;
  }

  .preview-text:hover {
    color: rgba(255, 255, 255, 1);
    background: rgba(255, 255, 255, 0.04);
  }

  .preview-textarea {
    flex: 1;
    font-size: 13.5px;
    line-height: 1.65;
    color: rgba(255, 255, 255, 0.92);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    padding: 8px;
    resize: none;
    outline: none;
    font-family: inherit;
    -webkit-user-select: text;
    user-select: text;
  }

  .preview-textarea:focus {
    border-color: rgba(167, 139, 250, 0.5);
  }

  .preview-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 10px;
    flex-shrink: 0;
  }

  .preview-hint {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    letter-spacing: 0.02em;
  }

  .preview-hint.editing-hint {
    color: rgba(167, 139, 250, 0.7);
  }

  .preview-btns {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preview-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    line-height: 1;
  }

  .preview-btn.cancel-btn {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .preview-btn.cancel-btn:hover {
    background: rgba(255, 59, 48, 0.2);
    border-color: rgba(255, 59, 48, 0.3);
    color: rgba(255, 255, 255, 0.85);
  }

  .preview-btn.confirm-btn {
    background: rgba(52, 199, 89, 0.2);
    color: rgba(52, 199, 89, 0.95);
    border: 1px solid rgba(52, 199, 89, 0.3);
  }

  .preview-btn.confirm-btn:hover {
    background: rgba(52, 199, 89, 0.3);
    border-color: rgba(52, 199, 89, 0.45);
    color: #fff;
  }

  .preview-btn:active {
    transform: scale(0.96);
  }

  /* ── Keycap styling ── */
  .keycap-group {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .keycap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.15);
    font-size: 10px;
    font-weight: 600;
    font-family: inherit;
    line-height: 1;
    color: inherit;
  }
</style>
