import { randomUUID } from 'node:crypto';
import {
  GLOBAL_RIGHTS_BASIS,
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
 * Deployment-wide rights policy. The operator confirms once, in private
 * deployment configuration, that they hold the rights to download and process
 * provider content. When enabled (the default), the server injects the same
 * attestation for every import, so a per-mixset confirmation is obsolete.
 */
export const GLOBAL_RIGHTS_ATTESTATION_VERSION = 'global-2026-11';

export function globalRightsConfirmationEnabled(
  value: string | undefined = process.env.IMPORT_GLOBAL_RIGHTS_CONFIRMATION,
): boolean {
  if (value === undefined || value === '') return true;
  return !/^(0|false|no|off)$/i.test(value.trim());
}

type ResolvedImportRequest = ImportRequest & {
  rights: NonNullable<ImportRequest['rights']>;
};

/**
 * Attach the effective rights attestation. With the global policy enabled the
 * server always sets it and a client-supplied basis cannot narrow or override
 * it. With the policy disabled, the request must carry its own basis.
 */
export function resolveRightsPolicy(
  request: ImportRequest,
  globalConfirmation = globalRightsConfirmationEnabled(),
): ResolvedImportRequest {
  if (!globalConfirmation) {
    if (!request.rights) {
      throw new ImportValidationError('A rights basis is required because the global rights confirmation is disabled', 400);
    }
    return request as ResolvedImportRequest;
  }
  return {
    ...request,
    rights: {
      basis: GLOBAL_RIGHTS_BASIS,
      attestationVersion: GLOBAL_RIGHTS_ATTESTATION_VERSION,
    },
  };
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

/**
 * The import plane is opt-in and must never run anonymously. It requires an
 * explicit request flag, a real scrypt curator hash and `AUTH_MODE=on`; with
 * `AUTH_MODE=off` every request authenticates as the curator, so enabling
 * imports there would expose acquisition without a login.
 */
export function resolveImportGate(input: {
  requested: string | undefined;
  passwordHash: string | undefined;
  authMode: 'on' | 'off';
}): boolean {
  return /^(1|true|yes|on)$/i.test(input.requested ?? '')
    && (input.passwordHash ?? '').startsWith('scrypt$')
    && input.authMode === 'on';
}

export { decideImportPolicy, PROVIDER_CAPABILITIES };
