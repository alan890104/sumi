import type {
  Settings,
  PolishConfig,
  SttConfig,
  CloudConfig,
  SttCloudConfig,
  PromptRule,
  DictionaryConfig,
  PolishModel,
  PolishMode,
  SttMode,
  CloudProvider,
  SttProvider,
  WhisperModelId,
} from '../types';
import * as api from '../api';

// ── Reactive settings state ──

let settings = $state<Settings>({
  hotkey: 'Alt+KeyZ',
  auto_paste: true,
  polish: {
    enabled: false,
    model: 'llama_taiwan',
    custom_prompt: null,
    mode: 'local',
    cloud: { provider: 'groq', api_key: '', endpoint: '', model_id: 'qwen/qwen3-32b' },
    prompt_rules: {},
    dictionary: { enabled: true, entries: [] },
    reasoning: false,
  },
  history_retention_days: 0,
  language: null,
  stt: {
    mode: 'local',
    cloud: { provider: 'deepgram', api_key: '', endpoint: '', model_id: 'whisper', language: 'auto' },
    whisper_model: 'large_v3_turbo',
    language: 'auto',
    vad_enabled: true,
  },
  edit_hotkey: null,
});

let onboardingCompleted = $state(true);

export function getSettings(): Settings {
  return settings;
}

export function getPolishConfig(): PolishConfig {
  return settings.polish;
}

export function getSttConfig(): SttConfig {
  return settings.stt;
}

export function getHotkey(): string {
  return settings.hotkey;
}

export function getEditHotkey(): string | null {
  return settings.edit_hotkey;
}

export function getOnboardingCompleted(): boolean {
  return onboardingCompleted;
}

export function setOnboardingCompleted(v: boolean) {
  onboardingCompleted = v;
}

// ── Load settings from backend ──

export async function load(): Promise<void> {
  const s = await api.getSettings();
  settings = s;

  // Check onboarding
  const saved = localStorage.getItem('sumi-onboarding-completed');
  onboardingCompleted = saved === 'true';

  // Load API keys from keychain
  try {
    settings.polish.cloud.api_key = await api.getApiKey(settings.polish.cloud.provider);
  } catch {
    settings.polish.cloud.api_key = '';
  }
  try {
    settings.stt.cloud.api_key = await api.getApiKey(`stt_${settings.stt.cloud.provider}`);
  } catch {
    settings.stt.cloud.api_key = '';
  }
}

// ── Save helpers ──

export function buildPayload(): Settings {
  const s = { ...settings };
  // Strip api_keys (stored in keychain, not in settings.json)
  s.polish = { ...s.polish, cloud: { ...s.polish.cloud, api_key: '' } };
  s.stt = { ...s.stt, cloud: { ...s.stt.cloud, api_key: '' } };
  return s;
}

export async function save(): Promise<void> {
  await api.saveSettings(buildPayload());
}

export async function savePolish(): Promise<void> {
  await save();
}

export async function saveStt(): Promise<void> {
  await save();
}

// ── Setters ──

export function setHotkey(hotkey: string) {
  settings.hotkey = hotkey;
}

export function setEditHotkey(hotkey: string | null) {
  settings.edit_hotkey = hotkey;
}

export function setLanguage(lang: string | null) {
  settings.language = lang;
}

export function setPolishEnabled(enabled: boolean) {
  settings.polish.enabled = enabled;
}

export function setPolishMode(mode: PolishMode) {
  settings.polish.mode = mode;
}

export function setPolishModel(model: PolishModel) {
  settings.polish.model = model;
}

export function setPolishReasoning(reasoning: boolean) {
  settings.polish.reasoning = reasoning;
}

export function setPolishCloudProvider(provider: CloudProvider) {
  settings.polish.cloud.provider = provider;
}

export function setPolishCloudApiKey(key: string) {
  settings.polish.cloud.api_key = key;
}

export function setPolishCloudEndpoint(endpoint: string) {
  settings.polish.cloud.endpoint = endpoint;
}

export function setPolishCloudModelId(modelId: string) {
  settings.polish.cloud.model_id = modelId;
}

export function setSttMode(mode: SttMode) {
  settings.stt.mode = mode;
}

export function setSttCloudProvider(provider: SttProvider) {
  settings.stt.cloud.provider = provider;
}

export function setSttCloudApiKey(key: string) {
  settings.stt.cloud.api_key = key;
}

export function setSttCloudEndpoint(endpoint: string) {
  settings.stt.cloud.endpoint = endpoint;
}

export function setSttCloudModelId(modelId: string) {
  settings.stt.cloud.model_id = modelId;
}

export function setSttLanguage(lang: string) {
  settings.stt.language = lang;
  settings.stt.cloud.language = lang;
}

export function setSttCloudLanguage(lang: string) {
  settings.stt.cloud.language = lang;
  settings.stt.language = lang;
}

export function setSttWhisperModel(model: WhisperModelId) {
  settings.stt.whisper_model = model;
}

export function setVadEnabled(v: boolean) {
  settings.stt.vad_enabled = v;
}

export function setHistoryRetention(days: number) {
  settings.history_retention_days = days;
}

export function setAutoPaste(v: boolean) {
  settings.auto_paste = v;
}

// ── Prompt rules ──

export function getCurrentRules(): PromptRule[] {
  // Flatten all language-keyed rules, deduplicating by match_type+match_value
  // (non-"auto" keys take priority since those are user-customized)
  const seen = new Map<string, PromptRule>();
  // Process "auto" first so it can be overwritten by customized rules
  for (const rule of settings.polish.prompt_rules['auto'] ?? []) {
    seen.set(`${rule.match_type}:${rule.match_value}`, rule);
  }
  for (const [key, rules] of Object.entries(settings.polish.prompt_rules)) {
    if (key === 'auto') continue;
    for (const rule of rules) {
      seen.set(`${rule.match_type}:${rule.match_value}`, rule);
    }
  }
  return Array.from(seen.values());
}

export function setCurrentRules(rules: PromptRule[]) {
  // Store all rules under "auto" key, clear others
  settings.polish.prompt_rules = { auto: rules };
}

// ── Dictionary ──

export function getDictionary(): DictionaryConfig {
  return settings.polish.dictionary;
}

export function setDictionaryEnabled(enabled: boolean) {
  settings.polish.dictionary.enabled = enabled;
}

export function setCustomPrompt(prompt: string | null) {
  settings.polish.custom_prompt = prompt;
}

export function markOnboardingComplete() {
  onboardingCompleted = true;
  localStorage.setItem('sumi-onboarding-completed', 'true');
}

export function resetOnboarding() {
  onboardingCompleted = false;
  localStorage.removeItem('sumi-onboarding-completed');
}
