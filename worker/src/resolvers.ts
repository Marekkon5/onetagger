import { boundedFetch } from './security.js';

/**
 * Resolver layer: each provider goes through its own adapter. SoundCloud is
 * metadata-only (official oEmbed): audio acquisition is always blocked by
 * policy both here (defense in depth) and on the control plane.
 */
export interface ResolvedSource {
  title?: string;
  artworkUrl?: string;
  providerId?: string;
  /** Direct, downloadable media URL — only when permitted for the provider. */
  audioUrl?: string;
  /** Private object key for a user-upload original. Never a filesystem path or fetch URL. */
  objectKey?: string;
}

export class NonRetryableError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'NonRetryableError';
  }
}

export function isUploadUrn(sourceUrl: string): boolean {
  return /^urn:syco23:upload:.+$/.test(sourceUrl);
}

export async function resolveSource(job: {
  provider: string;
  sourceUrl: string;
  mode: string;
  originalObjectKey?: string;
}): Promise<ResolvedSource> {
  if (job.provider === 'user_upload') {
    if (job.mode !== 'audio') {
      throw new NonRetryableError('User uploads are audio-only');
    }
    if (!job.originalObjectKey) {
      throw new NonRetryableError('User upload has no original object');
    }
    if (job.sourceUrl.startsWith('/') || job.sourceUrl.startsWith('file:') || job.sourceUrl.includes('://')) {
      throw new NonRetryableError('User upload source must be a private URN');
    }
    if (!isUploadUrn(job.sourceUrl)) {
      throw new NonRetryableError('User upload source must be a private URN');
    }
    return { objectKey: job.originalObjectKey };
  }
  if (job.provider === 'soundcloud') {
    if (job.mode === 'audio') {
      throw new NonRetryableError('Server-side SoundCloud audio acquisition is blocked by provider policy');
    }
    const oembedUrl = new URL('https://soundcloud.com/oembed');
    oembedUrl.search = new URLSearchParams({ format: 'json', url: job.sourceUrl }).toString();
    const { bytes } = await boundedFetch(oembedUrl.toString(), { maxBytes: 256 * 1024 });
    let payload: { title?: string; thumbnail_url?: string };
    try {
      payload = JSON.parse(bytes.toString('utf8'));
    } catch {
      throw new NonRetryableError('SoundCloud oEmbed returned unreadable metadata');
    }
    const artwork = payload.thumbnail_url && new URL(payload.thumbnail_url).hostname.endsWith('.sndcdn.com')
      ? payload.thumbnail_url
      : undefined;
    return { title: payload.title, artworkUrl: artwork };
  }
  if (job.provider === 'archiveorg' || job.provider === 'freeteknomusic') {
    if (isUploadUrn(job.sourceUrl) || job.sourceUrl.startsWith('/') || job.sourceUrl.startsWith('file:')) {
      throw new NonRetryableError('Provider audio must be an http(s) URL');
    }
    return job.mode === 'audio' ? { audioUrl: job.sourceUrl } : {};
  }
  throw new NonRetryableError(`No resolver registered for provider ${job.provider}`);
}
