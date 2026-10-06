import {
  CatalogRecordSchema,
  FieldClaimSchema,
  FieldDefinitions,
  ImportJobSchema,
  ProviderMetadataSchema,
  ScoredEvidenceSchema,
  validateField,
  type CatalogRecord,
  type FieldClaim,
  type ImportJob,
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
      evidence: metadata.match.status === 'confirmed' ? 'direct' : 'parsed',
      matchExplanation: metadata.match.explanation
    }));
  }

  return claims;
}

/** Convert import job worker evidence scores + artifacts into FieldClaims for merge policy. */
export function claimsFromImportJob(
  job: ImportJob,
  evidenceScores: ScoredEvidence[],
  artifactMap: Map<string, { role: string; objectKey: string }>,
  targetRecordId: string,
): FieldClaim[] {
  const claims: FieldClaim[] = [];
  if (!targetRecordId) return claims; // no linked record yet

  for (const scored of evidenceScores) {
    if (!scored.field) continue;
    if (!Object.hasOwn(FieldDefinitions, scored.field)) continue;

    // Build a claim value from the scored evidence
    // The worker evidence doesn't carry the actual value, so we infer from artifacts
    // and the job's source metadata.
    const value = inferClaimValue(scored.field, job, artifactMap);
    if (value === undefined) continue;

    const observedAt = scored.evaluatedAt;
    const evidence = scored.decision === 'auto_apply' ? 'direct' : scored.decision === 'review' ? 'parsed' : 'analysis';
    const matchExplanation = `Worker evidence (${scored.algorithmVersion}): score ${scored.score}, decision ${scored.decision}`;

    claims.push(FieldClaimSchema.parse({
      targetRecordId,
      field: scored.field,
      value,
      provider: { provider: job.provider, resourceType: 'recording', externalId: job.sourceExternalId ?? job.sourceUrl, url: job.sourceUrl.startsWith('urn:') ? undefined : job.sourceUrl },
      sourceUrl: job.sourceUrl.startsWith('urn:') ? `private://${job.sourceUrl}` : job.sourceUrl,
      observedAt,
      evidence,
      matchExplanation
    }));
  }

  return claims;
}

function inferClaimValue(field: string, job: ImportJob, artifactMap: Map<string, { role: string; objectKey: string }>): unknown {
  // Worker evidence scores don't include the actual field values.
  // For now we try to read the value from the metadata artifact.
  if (field === 'title') {
    const metaArtifact = Array.from(artifactMap.values()).find((a) => a.role === 'metadata');
    if (metaArtifact) {
      // objectKey is imports/<jobId>/metadata/<sha>.json; the artifact store
      // can be read through the repository. For the control plane we keep the
      // source metadata url as the provisional value; the merge policy will
      // pick it up and route to review when it needs real content.
      return job.sourceUrl;
    }
  }
  return undefined;
}