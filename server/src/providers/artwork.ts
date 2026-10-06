import { withTimeout } from '../core/utils.js';

const MAX_IMAGE_BYTES = 8 * 1024 * 1024;
const IMAGE_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp']);
const REDIRECTS = new Set([301, 302, 303, 307, 308]);

export class ArtworkFetchError extends Error {
  constructor(message: string, readonly status = 502) {
    super(message);
  }
}

function imageUrl(value: string): URL {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new ArtworkFetchError('Cover URL is invalid', 400);
  }
  const host = url.hostname.toLowerCase();
  const allowedHost = host === 'i.discogs.com'
    || host === 'i.ytimg.com'
    || host === 'img.youtube.com'
    || host === 'img.hearthis.at'
    || host === 'archive.org'
    || host.endsWith('.archive.org')
    || host.endsWith('.sndcdn.com');
  if (url.protocol !== 'https:' || url.username || url.password || url.port || !allowedHost) {
    throw new ArtworkFetchError('Cover URL host is not an approved provider image host', 400);
  }
  return url;
}

async function readImage(response: Response): Promise<Uint8Array> {
  const length = Number(response.headers.get('content-length'));
  if (Number.isFinite(length) && length > MAX_IMAGE_BYTES) {
    throw new ArtworkFetchError('Cover image exceeds the 8 MiB limit', 413);
  }
  if (!response.body) throw new ArtworkFetchError('Provider returned an empty cover image');
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > MAX_IMAGE_BYTES) {
      await reader.cancel();
      throw new ArtworkFetchError('Cover image exceeds the 8 MiB limit', 413);
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes;
}

export async function fetchProviderArtwork(value: string): Promise<{ bytes: Uint8Array; contentType: string }> {
  return withTimeout(async (signal) => {
    let url = imageUrl(value);
    for (let redirects = 0; redirects <= 3; redirects += 1) {
      const response = await fetch(url, {
        signal,
        redirect: 'manual',
        headers: { accept: 'image/jpeg,image/png,image/webp' },
      });
      if (REDIRECTS.has(response.status)) {
        const location = response.headers.get('location');
        if (!location || redirects === 3) throw new ArtworkFetchError('Provider image redirected too many times');
        url = imageUrl(new URL(location, url).toString());
        continue;
      }
      if (!response.ok) throw new ArtworkFetchError(`Provider image request failed (${response.status})`);
      const contentType = (response.headers.get('content-type') || '').split(';', 1)[0].trim().toLowerCase();
      if (!IMAGE_TYPES.has(contentType)) throw new ArtworkFetchError('Provider response is not a supported image');
      return { bytes: await readImage(response), contentType };
    }
    throw new ArtworkFetchError('Provider image redirected too many times');
  }, 10_000);
}