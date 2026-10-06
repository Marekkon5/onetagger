import { randomUUID } from 'node:crypto';
import {
  ImportRequestSchema,
  PROVIDER_CAPABILITIES,
  decideImportPolicy,
  type ImportRequest,
} from '@syco23/catalog-domain';

export class ImportValidationError extends Error {
  constructor(message: string, readonly statusCode = 400) {
    super(message);
  }
}

/**
 * Parse an import request body against the shared domain contract. The client and
 * server share this Zod schema, so a bad body fails the same way on both sides.
 */
export function parseImportRequest(input: unknown): ImportRequest {
  const parsed = ImportRequestSchema.safeParse(input);
  if (!parsed.success) {
    throw new ImportValidationError('Input validation failed', 400);
  }
  return parsed.data;
}

/**
 * Whether audio acquisition is permitted for a provider/rights pair.
 * Delegates to the domain policy so client previews and server enforcement can
 * never drift apart. Default deny: unknown providers never unlock audio.
 */
export function canAcquireAudio(provider: string, basis: string): boolean {
  const request = ImportRequestSchema.safeParse({
    source: { provider, url: 'https://example.com/track' },
    mode: 'audio',
    conversion: { format: 'mp3' },
    rights: { basis, attestationVersion: 'precheck' },
  });
  if (!request.success) return false;
  return decideImportPolicy(request.data).allowed;
}

export function newImportId(prefix = 'imp'): string {
  return `${prefix}_${randomUUID().replace(/-/g, '').slice(0, 24)}`;
}

export { decideImportPolicy, PROVIDER_CAPABILITIES };
