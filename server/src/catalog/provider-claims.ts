import {
  CatalogRecordSchema,
  FieldClaimSchema,
  FieldDefinitions,
  ImportJobSchema,
  ImportMetadataFactsSchema,
  ProviderMetadataSchema,
  RecordingDateSchema,
  ScoredEvidenceSchema,
  validateField,
  type CatalogRecord,
  type FieldClaim,
  type ImportJob,
  type ImportMetadataFacts,
  type ProviderMetadata,
  type ScoredEvidence
} from '@syco23/catalog-domain';

export type { ProviderMetadata };

export function claimsFromProvider(targetInput: CatalogRecord, metadataInput: ProviderMetadata): FieldClaim[] {
  const target = CatalogRecordSchema.parse(targetInput);
  const metadata = ProviderMetadataSchema.parse(metadataInput);
  const claims: FieldClaim[] = [];

  for (const [field, rawValue] of Object.entries(metadata.facts)) {
    if (!Object.hasOwn(FieldDefinitions, field)) continue;
    const value = validateField(target, field, rawValue);
    claims.push(FieldClaimSchema.parse({
      targetRecordId: target.id,
      field,
      value,
      provider: metadata.provider,
      sourceUrl: metadata.sourceUrl,
      observedAt: metadata.observedAt,
      evidence: metadata.fieldEvidence?.[field] === 'parsed' || metadata.match.status !== 'confirmed' ? 'parsed' : metadata.fieldEvidence?.[field] ?? 'direct',
      matchExplanation: metadata.match.explanation
    }));
  }

  return claims;
}

/** Convert import job worker evidence scores + sourced facts into FieldClaims for merge policy. */
export function claimsFromImportJob(
  job: ImportJob,
  evidenceScores: ScoredEvidence[],
  facts: ImportMetadataFacts | undefined,
  targetRecordId: string,
): FieldClaim[] {
  const claims: FieldClaim[] = [];
  if (!targetRecordId) return claims; // no linked record yet
  // A private upload has no public provider URL, so it cannot carry a provider
  // claim. Its title and facts are written directly by finalize.
  if (job.provider === 'user_upload' || job.sourceUrl.startsWith('urn:')) return claims;

  const parsedJob = ImportJobSchema.parse(job);
  const parsedFacts = facts ? ImportMetadataFactsSchema.parse(facts) : undefined;

  for (const scored of evidenceScores) {
    if (!scored.field) continue;
    if (!Object.hasOwn(FieldDefinitions, scored.field)) continue;
    // Rejected evidence is a gate failure, not a weak claim. It must never be
    // promoted into a claim or routed to Review.
    if (scored.decision === 'reject') continue;

    const value = claimValueFor(scored.field, scored, parsedFacts);
    if (value === undefined) continue;

    const evidence = scored.decision === 'auto_apply' ? 'direct' : 'parsed';
    const matchExplanation = `Worker evidence (${scored.algorithmVersion}): score ${scored.score}, decision ${scored.decision}`;

    claims.push(FieldClaimSchema.parse({
      targetRecordId,
      field: scored.field,
      value,
      provider: {
        provider: parsedJob.provider,
        resourceType: 'recording',
        externalId: parsedJob.sourceExternalId ?? parsedJob.sourceUrl,
        url: parsedJob.sourceUrl,
      },
      sourceUrl: parsedJob.sourceUrl,
      observedAt: scored.evaluatedAt,
      evidence,
      matchExplanation
    }));
  }

  return claims;
}

function recordingDate(value: string | undefined) {
  if (!value) return undefined;
  const precision = value.length === 4 ? 'year' : value.length === 7 ? 'month' : 'day';
  const parsed = RecordingDateSchema.safeParse({ value, precision });
  return parsed.success ? parsed.data : undefined;
}

/** Read the sourced value for a field, preferring the value submitted with the evidence. */
function claimValueFor(field: string, scored: ScoredEvidence, facts: ImportMetadataFacts | undefined): unknown {
  if (scored.value !== undefined) return scored.value;
  if (!facts) return undefined;
  switch (field) {
    case 'title': return facts.title;
    case 'description': return facts.description;
    case 'durationMs': return facts.durationMs;
    case 'genres': return facts.genres;
    case 'styles': return facts.styles;
    case 'recordingDate': return recordingDate(facts.recordedAt);
    case 'cover': return facts.artworkUrl ? { role: 'mix-cover', url: facts.artworkUrl } : undefined;
    default: return undefined;
  }
}