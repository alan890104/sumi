#!/usr/bin/env node
/**
 * sync-i18n.mjs — Keep all locale files in sync with en.json (source of truth).
 *
 * Usage:
 *   node scripts/sync-i18n.mjs            # Report coverage (exit 1 if gaps)
 *   node scripts/sync-i18n.mjs --fill     # Add missing keys with English fallback
 *   node scripts/sync-i18n.mjs --sort     # Reorder keys + remove stale extras
 *   node scripts/sync-i18n.mjs --fill --sort  # Both at once
 */

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const I18N_DIR = join(__dirname, "..", "frontend", "src", "i18n");

const args = process.argv.slice(2);
const doFill = args.includes("--fill");
const doSort = args.includes("--sort");

// ── Read en.json as raw text to capture section structure ──────────────
const enRaw = readFileSync(join(I18N_DIR, "en.json"), "utf-8");
const enData = JSON.parse(enRaw);
const enKeys = Object.keys(enData);

/**
 * Parse en.json to figure out which keys belong to which "section",
 * separated by blank lines in the source file.
 * Returns an array of arrays, each inner array is one section of keys.
 */
function parseEnSections() {
  const lines = enRaw.split("\n");
  const sections = [];
  let currentSection = [];

  for (const line of lines) {
    const trimmed = line.trim();
    // Match a JSON key-value pair
    const m = trimmed.match(/^"([^"]+)"\s*:/);
    if (m) {
      currentSection.push(m[1]);
    } else if (trimmed === "" && currentSection.length > 0) {
      // Blank line → end current section
      sections.push(currentSection);
      currentSection = [];
    }
  }
  if (currentSection.length > 0) {
    sections.push(currentSection);
  }
  return sections;
}

const sections = parseEnSections();

/**
 * Serialize a locale object to JSON, following en.json's section layout
 * and key order, with blank lines between sections.
 */
function serializeLocale(data) {
  const lines = ["{"];
  for (let si = 0; si < sections.length; si++) {
    const sectionKeys = sections[si];
    for (let ki = 0; ki < sectionKeys.length; ki++) {
      const key = sectionKeys[ki];
      const val = data[key];
      if (val === undefined) continue; // shouldn't happen after fill
      const isLast =
        si === sections.length - 1 && ki === sectionKeys.length - 1;
      const comma = isLast ? "" : ",";
      lines.push(`  ${JSON.stringify(key)}: ${JSON.stringify(val)}${comma}`);
    }
    // Add blank line between sections (not after the last one)
    if (si < sections.length - 1) {
      lines.push("");
    }
  }
  lines.push("}");
  return lines.join("\n") + "\n";
}

// ── Discover locale files ─────────────────────────────────────────────
const localeFiles = readdirSync(I18N_DIR)
  .filter((f) => f.endsWith(".json") && f !== "en.json")
  .sort();

// ── Process each locale ───────────────────────────────────────────────
let hasGaps = false;
const report = [];

for (const file of localeFiles) {
  const filePath = join(I18N_DIR, file);
  const locale = file.replace(".json", "");
  const data = JSON.parse(readFileSync(filePath, "utf-8"));
  const dataKeys = new Set(Object.keys(data));

  const missing = enKeys.filter((k) => !dataKeys.has(k));
  const extra = [...dataKeys].filter((k) => !enKeys.includes(k));

  report.push({ locale, total: enKeys.length, missing: missing.length, extra: extra.length });

  if (missing.length > 0) hasGaps = true;

  // Apply --fill: add missing keys with English value
  if (doFill) {
    for (const k of missing) {
      data[k] = enData[k];
    }
  }

  // Apply --sort: reorder to match en.json key order + remove extras
  if (doSort) {
    const sorted = {};
    for (const k of enKeys) {
      if (k in data) {
        sorted[k] = data[k];
      } else if (doFill) {
        sorted[k] = enData[k];
      }
    }
    writeFileSync(filePath, serializeLocale(sorted));
  } else if (doFill && missing.length > 0) {
    // Write back even without --sort, to persist filled keys
    // (append-style, preserving original order + adding missing at end)
    writeFileSync(filePath, JSON.stringify(data, null, 2) + "\n");
  }
}

// Also sort en.json itself for consistency
if (doSort) {
  writeFileSync(join(I18N_DIR, "en.json"), serializeLocale(enData));
}

// ── Print report ──────────────────────────────────────────────────────
const complete = report.filter((r) => r.missing === 0 && r.extra === 0).length;
const total = report.length + 1; // +1 for en.json itself

console.log(`\ni18n sync report (source: en.json, ${enKeys.length} keys)\n`);
console.log("Locale  Missing  Extra   Status");
console.log("──────  ───────  ─────   ──────");
for (const r of report) {
  const status =
    r.missing === 0 && r.extra === 0
      ? "✓"
      : r.missing > 0 && r.extra > 0
        ? `✗ (${r.missing} missing, ${r.extra} extra)`
        : r.missing > 0
          ? `✗ (${r.missing} missing)`
          : `✗ (${r.extra} extra)`;
  console.log(
    `${r.locale.padEnd(8)}${String(r.missing).padStart(5)}    ${String(r.extra).padStart(5)}   ${status}`
  );
}
console.log(`\n${complete + 1}/${total} complete (en.json + ${complete} locales)`);

if (!doFill && !doSort && hasGaps) {
  console.log(
    "\nRun with --fill --sort to add missing keys and normalize all files.\n"
  );
  process.exit(1);
}

if (doFill || doSort) {
  console.log(
    `\nApplied:${doFill ? " --fill" : ""}${doSort ? " --sort" : ""}\n`
  );
}
