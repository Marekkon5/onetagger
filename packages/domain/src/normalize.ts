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
  if (ref.provider === 'youtube' && ref.resourceType === 'video' && /(^|\.)((youtube\.com)|(youtu\.be))$/.test(url.hostname)) {
    const canonical = new URL('https://www.youtube.com/watch');
    canonical.searchParams.set('v', ref.externalId);
    // A timestamp/playlist identifies viewing context, not a different recording.
    return { ...ref, url: canonical.toString() };
  }
  if (ref.provider === 'soundcloud' && /(^|\.)soundcloud\.com$/.test(url.hostname)) { url.hostname = 'soundcloud.com'; url.protocol = 'https:'; url.pathname = url.pathname.replace(/\/$/, ''); }
  if (ref.provider === 'discogs' && /(^|\.)discogs\.com$/.test(url.hostname) && ['artist','label'].includes(ref.resourceType)) return { ...ref, url: `https://www.discogs.com/${ref.resourceType}/${ref.externalId}` };
  url.searchParams.sort();
  return { ...ref, url: url.toString() };
}