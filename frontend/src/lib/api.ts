import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Settings,
  MicStatus,
  ModelStatus,
  LlmModelStatus,
  PermissionStatus,
  HistoryEntry,
  DownloadProgress,
  TestPolishResult,
  GeneratedRule,
  PromptRule,
} from './types';

// ── Settings ──

export const getSettings = () => invoke<Settings>('get_settings');

export const saveSettings = (newSettings: Settings) =>
  invoke<void>('save_settings', { newSettings });

export const updateHotkey = (hotkey: string) =>
  invoke<void>('update_hotkey', { hotkey });

export const updateEditHotkey = (hotkey: string) =>
  invoke<void>('update_edit_hotkey', { hotkey });

export const resetSettings = () => invoke<void>('reset_settings');

export const getDefaultPrompt = () =>
  invoke<string>('get_default_prompt');

export const getDefaultPromptRules = () =>
  invoke<PromptRule[]>('get_default_prompt_rules');

// ── Recording ──

export const startRecording = () => invoke<void>('start_recording');

export const stopRecording = () => invoke<string>('stop_recording');

export const cancelRecording = () => invoke<void>('cancel_recording');

export const setTestMode = (enabled: boolean) =>
  invoke<void>('set_test_mode', { enabled });

export const setVoiceRuleMode = (enabled: boolean) =>
  invoke<void>('set_voice_rule_mode', { enabled });

export const setContextOverride = (appName: string, bundleId: string, url: string) =>
  invoke<void>('set_context_override', { appName, bundleId, url });

export const triggerUndo = () => invoke<void>('trigger_undo');

export const setEditTextOverride = (text: string) =>
  invoke<void>('set_edit_text_override', { text });

// ── History ──

export const getHistory = () => invoke<HistoryEntry[]>('get_history');

export const deleteHistoryEntry = (id: string) =>
  invoke<void>('delete_history_entry', { id });

export const exportHistoryAudio = (id: string) =>
  invoke<string>('export_history_audio', { id });

export const clearAllHistory = () => invoke<void>('clear_all_history');

export const getHistoryStoragePath = () => invoke<string>('get_history_storage_path');

export const getAppIcon = (bundleId: string) =>
  invoke<string>('get_app_icon', { bundleId });

// ── API Keys ──

export const saveApiKey = (provider: string, key: string) =>
  invoke<void>('save_api_key', { provider, key });

export const getApiKey = (provider: string) =>
  invoke<string>('get_api_key', { provider });

// ── Polish ──

export const testPolish = (testText: string, customPrompt: string) =>
  invoke<TestPolishResult>('test_polish', { testText, customPrompt });

export const generateRuleFromDescription = (description: string) =>
  invoke<GeneratedRule>('generate_rule_from_description', { description });

// ── Models ──

export const checkModelStatus = () => invoke<ModelStatus>('check_model_status');

export const downloadModel = () => invoke<void>('download_model');

export const checkLlmModelStatus = () => invoke<LlmModelStatus>('check_llm_model_status');

export const downloadLlmModel = () => invoke<void>('download_llm_model');

// ── Mic & Permissions ──

export const getMicStatus = () => invoke<MicStatus>('get_mic_status');

export const checkPermissions = () => invoke<PermissionStatus>('check_permissions');

export const openPermissionSettings = (permissionType: string) =>
  invoke<void>('open_permission_settings', { permissionType });

// ── Events ──

export const onRecordingStatus = (cb: (status: string) => void): Promise<UnlistenFn> =>
  listen<string>('recording-status', (e) => cb(e.payload));

export const onRecordingMaxDuration = (cb: (secs: number) => void): Promise<UnlistenFn> =>
  listen<number>('recording-max-duration', (e) => cb(e.payload));

export const onAudioLevels = (cb: (levels: number[]) => void): Promise<UnlistenFn> =>
  listen<number[]>('audio-levels', (e) => cb(e.payload));

export const onTranscriptionResult = (cb: (text: string) => void): Promise<UnlistenFn> =>
  listen<string>('transcription-result', (e) => cb(e.payload));

export const onHotkeyActivated = (cb: (v: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>('hotkey-activated', (e) => cb(e.payload));

export const onModelDownloadProgress = (
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> =>
  listen<DownloadProgress>('model-download-progress', (e) => cb(e.payload));

export const onLlmModelDownloadProgress = (
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> =>
  listen<DownloadProgress>('llm-model-download-progress', (e) => cb(e.payload));

export const onVoiceRuleStatus = (cb: (status: string) => void): Promise<UnlistenFn> =>
  listen<string>('voice-rule-status', (e) => cb(e.payload));

export const onVoiceRuleLevels = (cb: (levels: number[]) => void): Promise<UnlistenFn> =>
  listen<number[]>('voice-rule-levels', (e) => cb(e.payload));

export const onVoiceRuleTranscript = (cb: (text: string) => void): Promise<UnlistenFn> =>
  listen<string>('voice-rule-transcript', (e) => cb(e.payload));
