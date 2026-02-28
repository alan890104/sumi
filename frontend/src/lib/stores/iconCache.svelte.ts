import { getAppIcon } from '$lib/api';

let cache = $state<Record<string, string>>({});
const inflight = new Set<string>();

export function iconUri(bundleId: string): string | undefined {
  return cache[bundleId];
}

export async function resolveIcons(items: { bundle_id: string }[]) {
  const bundleIds = [...new Set(items.map((e) => e.bundle_id).filter(Boolean))];
  const missing = bundleIds.filter((bid) => !cache[bid] && !inflight.has(bid));
  if (missing.length === 0) return;

  for (const bid of missing) inflight.add(bid);

  const results = await Promise.allSettled(
    missing.map(async (bid) => {
      const uri = await getAppIcon(bid);
      return { bid, uri };
    }),
  );

  const newCache = { ...cache };
  for (const r of results) {
    if (r.status === 'fulfilled') {
      newCache[r.value.bid] = r.value.uri;
    }
  }
  for (const bid of missing) inflight.delete(bid);
  cache = newCache;
}
