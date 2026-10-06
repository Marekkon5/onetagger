import type { MixCandidate } from '../domain.js';

/**
 * File formats are technical noise and never carry identity.
 * `live`, `dj` and `set` are meaningful descriptors ("Live at Wacken", "DJ Koalisson",
 * "DJ set") and must survive normalization, because they distinguish one recording
 * from another. Only the `mix` family describes the uploaded artifact rather than
 * the recording, and is dropped in trailing position.
 */
const alwaysNoise = new Set(['mp3', 'flac', 'wav', 'ogg', 'aiff', 'm4a', 'aif', 'opus']);
const trailingNoise = new Set(['mix', 'mixset', 'mixsets']);

export function normalizeQuery(value = ''): string {
  const cleaned = value
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/https?:\/\/\S+/g, ' ')
    .replace(/[_./\\()[\]{}|:+-]+/g, ' ');
  const tokens = cleaned.split(/\s+/).filter(Boolean);
  const kept = tokens.filter((token, index) => {
    if (alwaysNoise.has(token)) return false;
    if (trailingNoise.has(token) && index === tokens.length - 1 && hasMeaningfulPrefix(tokens, index)) return false;
    return true;
  });
  return kept.join(' ').trim();
}

function hasMeaningfulPrefix(tokens: string[], index: number): boolean {
  // Only strip a trailing artifact word when something meaningful precedes it,
  // so a title that is literally just "Mix" is not emptied out.
  return tokens.slice(0, index).some((token) => !alwaysNoise.has(token) && !trailingNoise.has(token));
}

export function tokenSet(value = ''): Set<string> {
  return new Set(normalizeQuery(value).split(' ').filter((token) => token.length > 1));
}

export function overlapScore(left = '', right = ''): number {
  const a = tokenSet(left);
  const b = tokenSet(right);
  if (!a.size || !b.size) return 0;
  let hits = 0;
  for (const token of a) if (b.has(token)) hits += 1;
  return hits / Math.max(a.size, b.size);
}

export function confidenceScore(input: {
  query?: string;
  title?: string;
  artist?: string;
  durationExpectedMs?: number;
  durationActualMs?: number;
  pathContext?: string;
}): { score: number; reasons: string[] } {
  const reasons: string[] = [];
  let score = 0.28;

  const titleOverlap = overlapScore(input.query, input.title);
  if (titleOverlap) {
    score += Math.min(0.34, titleOverlap * 0.34);
    reasons.push(`title/query overlap ${Math.round(titleOverlap * 100)}%`);
  }

  const artistOverlap = overlapScore(input.query, input.artist);
  if (artistOverlap) {
    score += Math.min(0.18, Math.max(0.1, artistOverlap * 0.18));
    reasons.push('artist token match');
  }

  const pathOverlap = overlapScore(input.query, input.pathContext);
  if (pathOverlap >= 0.25) {
    score += Math.min(0.12, pathOverlap * 0.12);
    reasons.push('directory/context match');
  }

  if (input.durationExpectedMs && input.durationActualMs) {
    const delta = Math.abs(input.durationExpectedMs - input.durationActualMs) / input.durationExpectedMs;
    if (delta <= 0.08) {
      score += 0.14;
      reasons.push('duration within 8%');
    } else if (delta <= 0.2) {
      score += 0.07;
      reasons.push('duration within 20%');
    }
  }

  return { score: Math.min(0.99, Number(score.toFixed(3))), reasons };
}

export function duplicateKey(candidate: Pick<MixCandidate, 'title' | 'artists' | 'durationMs'>): string {
  const durationBucket = candidate.durationMs ? Math.round(candidate.durationMs / 30_000) : 0;
  return [normalizeQuery(candidate.artists.join(' ')), normalizeQuery(candidate.title), durationBucket].join('|');
}

export async function withTimeout<T>(work: (signal: AbortSignal) => Promise<T>, timeoutMs = 12_000, outer?: AbortSignal): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(new Error('timeout')), timeoutMs);
  const abort = () => controller.abort(outer?.reason);
  outer?.addEventListener('abort', abort, { once: true });
  try {
    return await work(controller.signal);
  } finally {
    clearTimeout(timeout);
    outer?.removeEventListener('abort', abort);
  }
}

export async function retry<T>(work: () => Promise<T>, attempts = 2): Promise<T> {
  let lastError: unknown;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      return await work();
    } catch (error) {
      lastError = error;
      if (attempt + 1 < attempts) await new Promise((resolve) => setTimeout(resolve, 250 * (attempt + 1)));
    }
  }
  throw lastError;
}

export function uniqueCandidates(candidates: MixCandidate[]): MixCandidate[] {
  const best = new Map<string, MixCandidate>();
  for (const candidate of candidates) {
    const key = duplicateKey(candidate);
    const current = best.get(key);
    if (!current || candidate.confidence > current.confidence) best.set(key, candidate);
  }
  return [...best.values()].sort((a, b) => b.confidence - a.confidence);
}
