/** Format bytes as human-readable size string (e.g. "1.5 GB", "800 MB"). */
export function formatSize(bytes: number): string {
  if (bytes >= 1_073_741_824) return (bytes / 1_073_741_824).toFixed(1) + ' GB';
  return (bytes / 1_048_576).toFixed(0) + ' MB';
}

/** Convert snake_case to camelCase (e.g. "large_v3_turbo" → "largeV3Turbo"). */
export function camelCase(id: string): string {
  return id.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
}
