import { z } from 'zod';
import { RecordIdSchema, TimestampSchema } from './schemas.js';

export const EvidenceAlgorithmVersionSchema = z.literal('evidence-v1');

export const EvidenceComponentsSchema = z.object({
  I: z.number().min(0).max(1),
  T: z.number().min(0).max(1),
  D: z.number().min(0).max(1),
  R: z.number().min(0).max(1),
  P: z.number().min(0).max(1),
  C: z.number().min(0).max(1),
  X: z.number().min(0).max(1),
}).strict();

export type EvidenceComponents = z.infer<typeof EvidenceComponentsSchema>;

export const EvidenceGatesSchema = z.object({
  identity: z.boolean(),
  rights: z.boolean(),
  noConflict: z.boolean(),
  policy: z.boolean(),
}).strict();

export type EvidenceGates = z.infer<typeof EvidenceGatesSchema>;

export const EvidenceDecisionSchema = z.enum(['auto_apply', 'review', 'reject']);

export type EvidenceDecision = z.infer<typeof EvidenceDecisionSchema>;

export const ScoredEvidenceSchema = z.object({
  id: RecordIdSchema.optional(),
  jobId: RecordIdSchema.optional(),
  claimId: RecordIdSchema.optional(),
  field: z.string().min(1).max(120).optional(),
  score: z.number().min(0).max(100),
  algorithmVersion: EvidenceAlgorithmVersionSchema,
  components: EvidenceComponentsSchema,
  hardGates: EvidenceGatesSchema,
  decision: EvidenceDecisionSchema,
  evaluatedAt: TimestampSchema,
}).strict();

export type ScoredEvidence = z.infer<typeof ScoredEvidenceSchema>;

export interface EvidenceInput extends EvidenceComponents {
  identityGate: boolean;
  rightsGate: boolean;
  noDirectConflict: boolean;
  providerPolicyAllowsUse: boolean;
}

function clampScore(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

/**
 * Deterministic field-level evidence score (evidence-v1).
 * E = clamp(0,100, 30I + 20T + 15D + 10R + 15P + 10C - 100X)
 * Components are 0..1. X is a conflict penalty (0 or 1 in v1).
 */
export function scoreEvidence(input: EvidenceInput, evaluatedAt = new Date().toISOString()): ScoredEvidence {
  const parsed = EvidenceComponentsSchema.parse({ I: input.I, T: input.T, D: input.D, R: input.R, P: input.P, C: input.C, X: input.X });
  const raw = 30 * parsed.I + 20 * parsed.T + 15 * parsed.D + 10 * parsed.R + 15 * parsed.P + 10 * parsed.C - 100 * parsed.X;
  const score = clampScore(raw);
  const gates: EvidenceGates = {
    identity: input.identityGate,
    rights: input.rightsGate,
    noConflict: input.noDirectConflict,
    policy: input.providerPolicyAllowsUse,
  };
  const gatesPass = gates.identity && gates.rights && gates.noConflict && gates.policy;
  const decision: EvidenceDecision = !gatesPass ? 'reject' : score >= 80 ? 'auto_apply' : 'review';
  // Conflict penalty forces rejection regardless of raw total.
  const finalDecision: EvidenceDecision = parsed.X >= 1 ? 'reject' : decision;
  return ScoredEvidenceSchema.parse({
    score,
    algorithmVersion: 'evidence-v1',
    components: parsed,
    hardGates: gates,
    decision: finalDecision,
    evaluatedAt,
  });
}

/** Duration component: 1.0 at <=2% deviation, linearly to 0 at >=10%. */
export function durationComponent(expectedMs: number | undefined, actualMs: number | undefined): number {
  if (!expectedMs || !actualMs || expectedMs <= 0 || actualMs <= 0) return 0;
  const deviation = Math.abs(actualMs - expectedMs) / expectedMs;
  if (deviation <= 0.02) return 1;
  if (deviation >= 0.1) return 0;
  return 1 - (deviation - 0.02) / 0.08;
}

export function autoApplyAllowed(scored: ScoredEvidence, threshold = 80): boolean {
  return scored.decision === 'auto_apply'
    && scored.score >= threshold
    && scored.hardGates.identity
    && scored.hardGates.rights
    && scored.hardGates.noConflict
    && scored.hardGates.policy;
}
