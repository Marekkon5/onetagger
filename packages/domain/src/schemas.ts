import countryCodes from 'i18n-iso-countries';
import { z } from 'zod';

export const ProviderIdSchema = z.enum([
  'freeteknomusic',
  'soundcloud',
  'archiveorg',
  'discogs',
  'youtube',
  'hearthis'
]);

export const EntityRoleSchema = z.enum(['artist', 'crew', 'label']);
export const IndexKindSchema = z.enum(['mix', 'artist', 'crew', 'label', 'event']);
export const VerificationSchema = z.enum(['proposed', 'source-confirmed', 'curator-confirmed']);
export const ReviewStateSchema = z.enum(['ready', 'review']);
export const RecordIdSchema = z.string().min(8).max(128).regex(/^[A-Za-z0-9_-]+$/);
export const TimestampSchema = z.iso.datetime({ offset: true });

const httpUrlSchema = z.url({ protocol: /^https?$/ }).max(2048);

export const ProviderRefSchema = z.object({
  provider: ProviderIdSchema,
  resourceType: z.string().min(1).max(80),
  externalId: z.string().min(1).max(300),
  url: httpUrlSchema.optional()
}).strict();

const dateParts = (value: string) => {
  const [year, month, day] = value.split('-').map(Number);
  if (month !== undefined && (month < 1 || month > 12)) return false;
  if (day !== undefined) {
    const candidate = new Date(Date.UTC(year, month - 1, day));
    return candidate.getUTCFullYear() === year && candidate.getUTCMonth() === month - 1 && candidate.getUTCDate() === day;
  }
  return true;
};

export const RecordingDateSchema = z.discriminatedUnion('precision', [
  z.object({ value: z.string().regex(/^\d{4}$/), precision: z.literal('year') }).strict(),
  z.object({ value: z.string().regex(/^\d{4}-\d{2}$/).refine(dateParts), precision: z.literal('month') }).strict(),
  z.object({ value: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(dateParts), precision: z.literal('day') }).strict()
]);

export const AssetRoleSchema = z.enum([
  'mix-cover',
  'artist-portrait',
  'crew-logo',
  'label-logo',
  'event-flyer',
  'waveform'
]);

export const MediaAssetSchema = z.object({
  role: AssetRoleSchema,
  url: z.union([httpUrlSchema, z.string().regex(/^\/api\/catalog\/media\/[A-Za-z0-9_-]{8,128}$/)]),
  source: z.union([ProviderIdSchema, z.enum(['curator', 'local'])]),
  width: z.number().int().positive().max(20000).optional(),
  height: z.number().int().positive().max(20000).optional(),
  mediaId: RecordIdSchema.optional()
}).strict();

export const ProviderSourceSchema = ProviderRefSchema.extend({
  addedAt: TimestampSchema.optional(),
  recordingDate: RecordingDateSchema.optional(),
  uploadDate: RecordingDateSchema.optional()
}).strict();

const RecordBaseSchema = z.object({
  id: RecordIdSchema,
  createdAt: TimestampSchema,
  updatedAt: TimestampSchema,
  revision: z.number().int().positive(),
  verification: VerificationSchema,
  reviewState: ReviewStateSchema,
  selectedEvidence: z.record(z.string(), RecordIdSchema).optional()
});

const displayText = z.string().min(1).max(500).refine((value) => value.trim().length > 0);
export const CountryCodeSchema = z.string().refine(
  (value) => /^[A-Z]{2}$/.test(value) && countryCodes.isValid(value),
  'Expected an ISO 3166-1 alpha-2 country code'
);

export const MixRecordSchema = RecordBaseSchema.extend({
  kind: z.literal('mix'),
  title: displayText,
  description: z.string().max(10000).optional(),
  durationMs: z.number().finite().int().positive().max(86400000).optional(),
  recordingDate: RecordingDateSchema.optional(),
  people: z.array(z.object({ entityId: RecordIdSchema, role: EntityRoleSchema }).strict()).max(100),
  eventIds: z.array(RecordIdSchema).max(100),
  genres: z.array(displayText).max(100),
  styles: z.array(displayText).max(100),
  assets: z.array(MediaAssetSchema).max(100),
  sources: z.array(ProviderSourceSchema).max(100),
  playbackUrls: z.array(httpUrlSchema).max(100).optional()
}).strict();

const ArtistDetailsSchema = z.object({
  realName: displayText.optional(),
  profile: z.string().max(20000).optional(),
  country: CountryCodeSchema.optional(),
  groupIds: z.array(RecordIdSchema).max(100).optional(),
  memberIds: z.array(RecordIdSchema).max(100).optional()
}).strict();

const CrewDetailsSchema = z.object({
  profile: z.string().max(20000).optional(),
  country: CountryCodeSchema.optional(),
  websiteUrls: z.array(httpUrlSchema).max(100).optional(),
  memberIds: z.array(RecordIdSchema).max(100).optional()
}).strict();

const LabelDetailsSchema = z.object({
  profile: z.string().max(20000).optional(),
  country: CountryCodeSchema.optional(),
  websiteUrls: z.array(httpUrlSchema).max(100).optional(),
  parentId: RecordIdSchema.optional(),
  subLabelIds: z.array(RecordIdSchema).max(100).optional()
}).strict();

export const EntityRecordSchema = RecordBaseSchema.extend({
  kind: z.literal('entity'),
  displayName: displayText,
  roles: z.array(EntityRoleSchema).min(1).max(3),
  aliases: z.array(displayText).max(100),
  profile: z.string().max(20000).optional(),
  country: CountryCodeSchema.optional(),
  assets: z.array(MediaAssetSchema).max(100),
  providerRefs: z.array(ProviderRefSchema).max(100),
  artist: ArtistDetailsSchema.optional(),
  crew: CrewDetailsSchema.optional(),
  label: LabelDetailsSchema.optional()
}).strict().superRefine((record, context) => {
  for (const role of ['artist', 'crew', 'label'] as const) {
    if (record[role] && !record.roles.includes(role)) {
      context.addIssue({ code: 'custom', path: [role], message: `Details require the ${role} role` });
    }
  }
});

export const EventRecordSchema = RecordBaseSchema.extend({
  kind: z.literal('event'),
  name: displayText,
  startDate: RecordingDateSchema.optional(),
  endDate: RecordingDateSchema.optional(),
  venue: displayText.optional(),
  locality: displayText.optional(),
  country: CountryCodeSchema.optional(),
  assets: z.array(MediaAssetSchema).max(100),
  sourceUrls: z.array(httpUrlSchema).max(100),
  mixIds: z.array(RecordIdSchema).max(1000)
}).strict();

export const CatalogRecordSchema = z.discriminatedUnion('kind', [
  MixRecordSchema,
  EntityRecordSchema,
  EventRecordSchema
]);

export const FieldClaimSchema = z.object({
  targetRecordId: RecordIdSchema,
  field: z.string().min(1).max(120),
  value: z.unknown(),
  provider: ProviderRefSchema,
  sourceUrl: httpUrlSchema,
  observedAt: TimestampSchema,
  evidence: z.enum(['direct', 'parsed', 'curated', 'analysis']),
  matchExplanation: z.string().min(1).max(2000),
  confidence: z.number().min(0).max(1).optional()
}).strict();

export const ReviewItemSchema = z.object({
  id: RecordIdSchema,
  targetRecordId: RecordIdSchema,
  field: z.string().min(1).max(120),
  claim: FieldClaimSchema,
  state: z.enum(['pending', 'accepted', 'rejected']),
  createdAt: TimestampSchema,
  recordRevision: z.number().int().positive(),
  currentValue: z.unknown().optional()
}).strict();

export const ClaimDispositionSchema = z.enum(['selected', 'corroborated', 'pending', 'rejected']);

/**
 * A stored claim together with what the merge policy did with it. This is what lets
 * the detail view answer "why does this field hold this value?" instead of only
 * showing a bare evidence id: a selected value can cite its direct source, a
 * corroborated one names the agreeing source, and a rejected one stays visible as a
 * decision rather than silently disappearing.
 */
export const FieldEvidenceSchema = z.object({
  id: RecordIdSchema,
  fingerprint: z.string().min(1).max(200),
  claim: FieldClaimSchema,
  disposition: ClaimDispositionSchema
}).strict();

export const FieldEvidenceListSchema = z.object({
  recordId: RecordIdSchema,
  evidence: z.array(FieldEvidenceSchema)
}).strict();

export const RelatedMixesSchema = z.object({
  recordId: RecordIdSchema,
  mixes: z.array(CatalogRecordSchema)
}).strict();

export const ProviderMetadataSchema = z.object({
  provider: ProviderRefSchema,
  sourceUrl: httpUrlSchema,
  observedAt: TimestampSchema,
  match: z.object({
    status: z.enum(['confirmed', 'possible']),
    explanation: z.string().min(1).max(2000)
  }).strict(),
  facts: z.record(z.string().max(120), z.unknown()).refine((facts) => Object.keys(facts).length <= 40)
}).strict();

export const LegacyMixSchema = z.object({
  id: z.string().min(1).max(200),
  title: z.string().min(1).max(500),
  artists: z.array(z.string()).optional(),
  crews: z.array(z.string()).optional(),
  durationMs: z.number().finite().positive().optional(),
  recordedAt: z.string().optional(),
  artwork: z.array(z.unknown()).optional(),
  sources: z.array(z.unknown()).optional(),
  entities: z.array(z.unknown()).optional(),
  waveform: z.unknown().optional()
}).passthrough();

export const PageQuerySchema = z.object({
  page: z.number().int().positive().default(1),
  pageSize: z.number().int().positive().max(50).default(25),
  query: z.string().max(200).optional()
}).strict();

export const CatalogPageSchema = z.object({
  items: z.array(CatalogRecordSchema),
  page: z.number().int().positive(),
  pageSize: z.number().int().positive().max(50),
  total: z.number().int().nonnegative()
}).strict();

export const ImportResultSchema = z.object({
  record: CatalogRecordSchema,
  outcome: z.enum(['created', 'existing', 'review'])
}).strict();

export const ImportCandidateSchema = z.object({
  provider: ProviderIdSchema,
  title: z.string().min(1).max(500),
  artists: z.array(z.string().max(500)).max(100).default([]),
  crews: z.array(z.string().max(500)).max(100).default([]),
  durationMs: z.number().finite().int().positive().max(86400000).optional(),
  recordedAt: z.string().max(40).optional(),
  description: z.string().max(10000).optional(),
  genres: z.array(z.string().max(500)).max(100).default([]),
  artwork: z.array(httpUrlSchema).max(10).default([]),
  source: ProviderRefSchema.extend({ provider: ProviderIdSchema, url: httpUrlSchema }).strict(),
  confidence: z.number().min(0).max(1),
  reasons: z.array(z.string().max(1000)).max(30).default([])
}).strict().refine((candidate) => candidate.provider === candidate.source.provider, {
  message: 'Candidate provider must match its source identity',
  path: ['source', 'provider']
});

export const EnrichmentReportSchema = z.object({
  state: z.enum(['completed', 'interrupted']),
  attemptedProviders: z.array(ProviderIdSchema),
  applied: z.number().int().nonnegative(),
  corroborated: z.number().int().nonnegative(),
  reviewed: z.number().int().nonnegative(),
  errors: z.array(z.object({ provider: ProviderIdSchema, message: z.string().max(2000) }).strict()),
  missingFields: z.array(z.string().max(120))
}).strict();

export const MigrationResultSchema = z.object({
  batchId: z.string().min(1).max(200),
  imported: z.number().int().nonnegative(),
  existing: z.number().int().nonnegative(),
  rejected: z.number().int().nonnegative(),
  legacyIds: z.record(z.string(), RecordIdSchema)
}).strict();

export type RecordId = z.infer<typeof RecordIdSchema>;
export type EntityRole = z.infer<typeof EntityRoleSchema>;
export type IndexKind = z.infer<typeof IndexKindSchema>;
export type ProviderRef = z.infer<typeof ProviderRefSchema>;
export type MixRecord = z.infer<typeof MixRecordSchema>;
export type EntityRecord = z.infer<typeof EntityRecordSchema>;
export type EventRecord = z.infer<typeof EventRecordSchema>;
export type CatalogRecord = z.infer<typeof CatalogRecordSchema>;
export type FieldClaim = z.infer<typeof FieldClaimSchema>;
export type ProviderMetadata = z.infer<typeof ProviderMetadataSchema>;
export type LegacyMix = z.infer<typeof LegacyMixSchema>;
export type PageQuery = z.infer<typeof PageQuerySchema>;
export type CatalogPage = z.infer<typeof CatalogPageSchema>;
export type ImportResult = z.infer<typeof ImportResultSchema>;
export type ImportCandidate = z.infer<typeof ImportCandidateSchema>;
export type EnrichmentReport = z.infer<typeof EnrichmentReportSchema>;
export type MigrationResult = z.infer<typeof MigrationResultSchema>;
export type CatalogDetail = CatalogRecord;
export type ReviewItem = z.infer<typeof ReviewItemSchema>;
export type ClaimDisposition = z.infer<typeof ClaimDispositionSchema>;
export type FieldEvidence = z.infer<typeof FieldEvidenceSchema>;
export type FieldEvidenceList = z.infer<typeof FieldEvidenceListSchema>;
export type RelatedMixes = z.infer<typeof RelatedMixesSchema>;