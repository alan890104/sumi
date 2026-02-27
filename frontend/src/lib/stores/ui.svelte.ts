import type { Page } from '../types';

// ── Current page ──

let currentPage = $state<Page>('stats');

export function getCurrentPage(): Page {
  return currentPage;
}

export function setCurrentPage(page: Page) {
  currentPage = page;
}

// ── Confirm modal ──

interface ConfirmState {
  visible: boolean;
  title: string;
  message: string;
  okLabel: string;
  onConfirm: (() => void) | null;
}

let confirm = $state<ConfirmState>({
  visible: false,
  title: '',
  message: '',
  okLabel: 'OK',
  onConfirm: null,
});

export function getConfirm(): ConfirmState {
  return confirm;
}

export function showConfirm(title: string, message: string, okLabel: string, onConfirm: () => void) {
  confirm = { visible: true, title, message, okLabel, onConfirm };
}

export function hideConfirm() {
  confirm = { ...confirm, visible: false, onConfirm: null };
}

// ── Setup overlay ──

let showSetup = $state(false);

export function getShowSetup(): boolean {
  return showSetup;
}

export function setShowSetup(v: boolean) {
  showSetup = v;
}

// ── Prompt rules ──

let expandedRuleIndex = $state(-1);

export function getExpandedRuleIndex(): number {
  return expandedRuleIndex;
}

export function setExpandedRuleIndex(index: number) {
  expandedRuleIndex = index;
}

export function toggleRuleExpand(index: number) {
  expandedRuleIndex = expandedRuleIndex === index ? -1 : index;
}

// ── Highlight section ──

let highlightSection = $state<string | null>(null);

export function getHighlightSection(): string | null {
  return highlightSection;
}

export function setHighlightSection(section: string | null) {
  highlightSection = section;
}
