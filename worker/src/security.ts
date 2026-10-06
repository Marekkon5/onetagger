import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';

export class UrlBlockedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'UrlBlockedError';
  }
}

const BLOCKED_HOSTNAMES = new Set(['localhost', 'localhost.localdomain', 'metadata.google.internal']);

function ipv4Blocked(ip: string): boolean {
  const parts = ip.split('.').map(Number);
  const [a, b] = parts;
  if (a === 0 || a === 10 || a === 127) return true;
  if (a === 169 && b === 254) return true; // link-local + cloud metadata
  if (a === 172 && b >= 16 && b <= 31) return true; // RFC1918
  if (a === 192 && b === 168) return true; // RFC1918
  if (a === 100 && b >= 64 && b <= 127) return true; // CGNAT
  if (a === 198 && (b === 18 || b === 19)) return true; // benchmarking
  return false;
}

function ipv6Blocked(ip: string): boolean {
  const lower = ip.toLowerCase();
  if (lower === '::' || lower === '::1') return true;
  if (lower.startsWith('fe80') || lower.startsWith('fc') || lower.startsWith('fd')) return true; // link-local / ULA
  if (lower.startsWith('::ffff:')) {
    const mapped = lower.slice('::ffff:'.length);
    if (isIP(mapped) === 4) return ipv4Blocked(mapped);
  }
  return false;
}

export function ipBlocked(ip: string): boolean {
  const version = isIP(ip);
  if (version === 4) return ipv4Blocked(ip);
  if (version === 6) return ipv6Blocked(ip);
  return true;
}

/**
 * Static URL guard: scheme, credentials, port and hostname shape.
 * Every fetch target (including each redirect hop) must pass through this.
 */
export function assertSafeUrl(raw: string): URL {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    throw new UrlBlockedError('URL is invalid');
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') throw new UrlBlockedError('Only http/https URLs are allowed');
  if (url.username || url.password) throw new UrlBlockedError('Credentials in URLs are not allowed');
  if (url.port && !['80', '443'].includes(url.port)) throw new UrlBlockedError('Only default ports are allowed');
  const host = url.hostname.replace(/^\[|\]$/g, '').toLowerCase();
  if (BLOCKED_HOSTNAMES.has(host)) throw new UrlBlockedError('Hostname is blocked');
  if (isIP(host) && ipBlocked(host)) throw new UrlBlockedError('IP address is blocked');
  return url;
}

/** Resolve DNS and reject private/loopback/link-local/metadata answers. */
export async function assertResolvedSafe(raw: string): Promise<URL> {
  const url = assertSafeUrl(raw);
  const host = url.hostname.replace(/^\[|\]$/g, '');
  if (isIP(host)) return url;
  let addresses;
  try {
    addresses = await lookup(host, { all: true });
  } catch {
    throw new UrlBlockedError('Hostname could not be resolved');
  }
  if (!addresses.length) throw new UrlBlockedError('Hostname could not be resolved');
  for (const entry of addresses) {
    if (ipBlocked(entry.address)) throw new UrlBlockedError('Hostname resolves to a blocked address');
  }
  return url;
}

/**
 * Bounded fetch with manual redirect handling: every hop is re-validated, the
 * hop count is capped and the body stream is capped at maxBytes.
 */
export async function boundedFetch(rawUrl: string, options: {
  maxBytes: number;
  maxRedirects?: number;
  timeoutMs?: number;
  headers?: Record<string, string>;
} ): Promise<{ bytes: Buffer; contentType: string; finalUrl: string }> {
  const maxRedirects = options.maxRedirects ?? 3;
  const timeoutMs = options.timeoutMs ?? 30_000;
  let current = rawUrl;
  for (let hop = 0; hop <= maxRedirects; hop++) {
    const url = await assertResolvedSafe(current);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetch(url, {
        redirect: 'manual',
        signal: controller.signal,
        headers: options.headers,
      });
      if ([301, 302, 303, 307, 308].includes(response.status)) {
        const location = response.headers.get('location');
        if (!location) throw new UrlBlockedError('Redirect without location');
        current = new URL(location, url).toString();
        continue;
      }
      if (!response.ok) throw new UrlBlockedError(`Upstream responded ${response.status}`);
      const declared = Number(response.headers.get('content-length') ?? '0');
      if (declared > options.maxBytes) throw new UrlBlockedError('Content-Length exceeds the input limit');
      const chunks: Buffer[] = [];
      let size = 0;
      for await (const chunk of response.body as unknown as AsyncIterable<Uint8Array>) {
        size += chunk.length;
        if (size > options.maxBytes) throw new UrlBlockedError('Body exceeds the input limit');
        chunks.push(Buffer.from(chunk));
      }
      return {
        bytes: Buffer.concat(chunks),
        contentType: response.headers.get('content-type') ?? 'application/octet-stream',
        finalUrl: url.toString(),
      };
    } finally {
      clearTimeout(timer);
    }
  }
  throw new UrlBlockedError('Too many redirects');
}
