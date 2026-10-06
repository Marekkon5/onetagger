import { z } from 'zod';
import { ProviderIdSchema, RecordIdSchema, TimestampSchema } from './schemas.js';

const httpUrlSchema = z.url({ protocol: /^https?$/ }).max(2048);
/** Provider jobs store an http(s) URL; user uploads store a private URN, never a filesystem path. */
export const ImportSourceUrlSchema = z.union([
  httpUrlSchema,
  z.string().min(18).max(256).regex(/^urn:syco23:upload:.+$/),
]);
export type ImportSourceUrl = z.infer<typeof ImportSourceUrlSchema>;


/** Extended provider set for the import control plane (catalog providers + user uploads). */
export const ImportProviderSchema = z.union([
  ProviderIdSchema,
  z.literal('user_upload'),
]);

export type ImportProvider = z.infer<typeof ImportProviderSchema>;

export const ImportModeSchema = z.enum(['metadata', 'audio']);

export type ImportMode = z.infer<typeof ImportModeSchema>;

export const ImportJobStateSchema = z.enum([
  'created',
  'policy_check',
  'queued',
  'resolving',
  'acquiring',
  'probing',
  'converting',
  'tagging',
  'enriching',
  'scoring',
  'review',
  'completed',
  'blocked_policy',
  'failed',
  'cancelled',
]);

export type ImportJobState = z.infer<typeof ImportJobStateSchema>;

export const IdempotencyKeySchema = z.string().min(8).max(128).regex(/^[A-Za-z0-9_-]+$/);

export type IdempotencyKey = z.infer<typeof IdempotencyKeySchema>;

export const RightsBasisSchema = z.enum([
  'provider_metadata_only',
  'user_authorized_copy',
  'licensed_archive',
  'rights_holder',
  'separate_agreement',
]);

export type RightsBasis = z.infer<typeof RightsBasisSchema>;

export const ProviderCapabilitySchema = z.object({
  metadata: z.boolean(),
  audio: z.boolean()
}).strict();

export type ProviderCapability = z.infer<typeof ProviderCapabilitySchema>;

export const ProviderCapabilityMatrixSchema = z.object({
  freeteknomusic: ProviderCapabilitySchema,
  soundcloud: ProviderCapabilitySchema,
  archiveorg: ProviderCapabilitySchema,
  discogs: ProviderCapabilitySchema,
  youtube: ProviderCapabilitySchema,
  hearthis: ProviderCapabilitySchema
}).strict();

export type ProviderCapabilityMatrix = z.infer<typeof ProviderCapabilityMatrixSchema>;

/**
 * Which providers may supply mix metadata vs. downloadable audio.
 *
 * Streaming-only origins (SoundCloud, YouTube, hearthis.at) expose metadata,
 * artwork and duration through public APIs/oEmbed but no permitted audio
 * download, so audio stays false. Archive-style origins expose direct audio
 * files for duration probing and permitted analysis. Discogs only enriches
 * entity profiles and never supplies mix audio.
 */
export const PROVIDER_CAPABILITIES = ProviderCapabilityMatrixSchema.parse({
  freeteknomusic: { metadata: true, audio: true },
  soundcloud: { metadata: true, audio: false },
  archiveorg: { metadata: true, audio: true },
  discogs: { metadata: true, audio: false },
  youtube: { metadata: true, audio: false },
  hearthis: { metadata: true, audio: false }
});

export const ConversionSchema = z.object({
  format: z.literal('mp3'),
  bitrateKbps: z.number().int().min(64).max(320).default(320),
  id3Version: z.literal('2.3').default('2.3'),
}).strict();

export type Conversion = z.infer<typeof ConversionSchema>;

export const ImportRequestSchema = z.object({
  source: z.object({
    provider: ImportProviderSchema,
    url: httpUrlSchema.optional(),
    uploadId: z.string().min(1).max(200).optional(),
    externalId: z.string().min(1).max(300).optional(),
  }).strict(),
  mode: ImportModeSchema,
  conversion: ConversionSchema.optional(),
  autoApplyThreshold: z.number().min(0).max(100).default(80),
  rights: z.object({
    basis: RightsBasisSchema,
    attestationVersion: z.string().min(1).max(40),
    proofObjectKey: z.string().min(1).max(1024).optional(),
  }).strict(),
}).strict().superRefine((value, ctx) => {
  if (value.source.provider === 'user_upload' && !value.source.uploadId) {
    ctx.addIssue({ code: 'custom', message: 'user_upload requires source.uploadId', path: ['source', 'uploadId'] });
  }
  if (value.source.provider !== 'user_upload' && !value.source.url) {
    ctx.addIssue({ code: 'custom', message: 'provider imports require source.url', path: ['source', 'url'] });
  }
  if (value.mode === 'audio' && !value.conversion) {
    ctx.addIssue({ code: 'custom', message: 'audio mode requires conversion', path: ['conversion'] });
  }
});

export type ImportRequest = z.infer<typeof ImportRequestSchema>;

export interface ProviderCapabilities {
  metadata: boolean;
  serverSideAudioAcquisition: boolean;
  requiresUserConsent: boolean;
  requiresExplicitRightsBasis: boolean;
}

function toCapabilities(provider: string): ProviderCapabilities {
  if (provider === 'user_upload') {
    return { metadata: false, serverSideAudioAcquisition: true, requiresUserConsent: true, requiresExplicitRightsBasis: true };
  }
  const legacy = (PROVIDER_CAPABILITIES as Record<string, { metadata: boolean; audio: boolean }>)[provider];
  if (!legacy) {
    return { metadata: false, serverSideAudioAcquisition: false, requiresUserConsent: false, requiresExplicitRightsBasis: true };
  }
  const serverSideAudioAcquisition = legacy.audio;
  return {
    metadata: legacy.metadata,
    serverSideAudioAcquisition,
    requiresUserConsent: false,
    // SoundCloud + streaming providers always need an explicit rights basis before audio.
    requiresExplicitRightsBasis: true,
  };
}

/** Policy decision for an import request: metadata-only, allowed audio, or blocked. */
export function decideImportPolicy(request: ImportRequest): { allowed: boolean; blockedPolicy: boolean; capabilities: ProviderCapabilities; reason?: string } {
  const capabilities = toCapabilities(request.source.provider);
  if (request.mode === 'metadata') {
    if (!capabilities.metadata && request.source.provider !== 'user_upload') {
      return { allowed: false, blockedPolicy: true, capabilities, reason: `Provider ${request.source.provider} does not expose metadata import` };
    }
    return { allowed: true, blockedPolicy: false, capabilities };
  }
  // Audio mode: capability + explicit non-metadata-only rights basis required.
  if (!capabilities.serverSideAudioAcquisition) {
    return { allowed: false, blockedPolicy: true, capabilities, reason: `Server-side audio acquisition is disabled for ${request.source.provider}` };
  }
  if (request.rights.basis === 'provider_metadata_only') {
    return { allowed: false, blockedPolicy: true, capabilities, reason: 'Audio imports require an explicit audio rights basis' };
  }
  return { allowed: true, blockedPolicy: false, capabilities };
}

export const ImportJobSchema = z.object({
  id: RecordIdSchema,
  requestedBy: z.string().min(1).max(200),
  provider: ImportProviderSchema,
  sourceUrl: ImportSourceUrlSchema,
  sourceExternalId: z.string().min(1).max(300).optional(),
  mode: ImportModeSchema,
  state: ImportJobStateSchema,
  idempotencyKey: IdempotencyKeySchema,
  mixId: RecordIdSchema.optional(),
  attempt: z.number().int().nonnegative().max(1000).default(0),
  errorCode: z.string().min(1).max(120).optional(),
  error: z.string().max(2000).optional(),
  createdAt: TimestampSchema,
  startedAt: TimestampSchema.optional(),
  finishedAt: TimestampSchema.optional(),
  updatedAt: TimestampSchema,
}).strict();

export type ImportJob = z.infer<typeof ImportJobSchema>;

export function terminalImportState(state: ImportJobState): boolean {
  return state === 'completed' || state === 'blocked_policy' || state === 'failed' || state === 'cancelled';
}

export function retryableImportState(state: ImportJobState): boolean {
  return state === 'failed' || state === 'cancelled';
}
