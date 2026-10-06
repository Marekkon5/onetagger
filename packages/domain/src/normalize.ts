import { ProviderRefSchema, type ProviderRef } from './schemas.js';

const trackingParameters = new Set(['fbclid', 'gclid', 'mc_cid', 'mc_eid']);

export function normalizeName(value: string): string {
  return value.normalize('NFKC').trim().replace(/\s+/gu, ' ').toLocaleLowerCase('und');
}

export function normalizeProviderRef(input: ProviderRef): ProviderRef {
  const ref = ProviderRefSchema.parse(input);
  if (!ref.url) return ref;

  const url = new URL(ref.url);
  for (const key of [...url.searchParams.keys()]) {
    if (key.toLocaleLowerCase('en-US').startsWith('utm_') || trackingParameters.has(key.toLocaleLowerCase('en-US'))) {
      url.searchParams.delete(key);
    }
  }
  url.hash = '';
  url.searchParams.sort();
  return { ...ref, url: url.toString() };
}