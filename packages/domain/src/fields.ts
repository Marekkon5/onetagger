import { z } from 'zod';
import {
  CatalogRecordSchema,
  CountryCodeSchema,
  RecordingDateSchema,
  type CatalogRecord,
  type EntityRecord,
  type EventRecord,
  type MixRecord
} from './schemas.js';

const textValue = z.string().min(1).max(10000);
const textListValue = z.array(z.string().min(1).max(500)).max(100);

export const FieldDefinitions = {
  title: { targets: ['mix'], schema: textValue },
  description: { targets: ['mix'], schema: z.string().max(10000) },
  durationMs: { targets: ['mix'], schema: z.number().finite().int().positive().max(86400000) },
  recordingDate: { targets: ['mix'], schema: RecordingDateSchema },
  genres: { targets: ['mix'], schema: textListValue },
  styles: { targets: ['mix'], schema: textListValue },
  cover: { targets: ['mix'], schema: z.object({ role: z.literal('mix-cover'), url: z.url() }).strict() },
  artistPortrait: { targets: ['entity'], schema: z.object({ role: z.literal('artist-portrait'), url: z.url() }).strict() },
  crewLogo: { targets: ['entity'], schema: z.object({ role: z.literal('crew-logo'), url: z.url() }).strict() },
  labelLogo: { targets: ['entity'], schema: z.object({ role: z.literal('label-logo'), url: z.url() }).strict() },
  displayName: { targets: ['entity'], schema: textValue },
  name: { targets: ['event'], schema: textValue },
  profile: { targets: ['entity'], schema: z.string().max(20000) },
  country: { targets: ['entity', 'event'], schema: CountryCodeSchema },
  eventDate: { targets: ['event'], schema: RecordingDateSchema },
  venue: { targets: ['event'], schema: textValue },
  flyer: { targets: ['event'], schema: z.object({ role: z.literal('event-flyer'), url: z.url() }).strict() }
} as const;

export type FieldName = keyof typeof FieldDefinitions;

export function validateField(record: CatalogRecord, field: string, value: unknown): unknown {
  const definition = FieldDefinitions[field as FieldName];
  if (!definition) throw new Error(`Unsupported catalog field: ${field}`);

  const target = record.kind === 'entity' ? 'entity' : record.kind;
  if (!(definition.targets as readonly string[]).includes(target)) {
    throw new Error(`Field ${field} does not apply to ${target} records`);
  }
  if (record.kind === 'entity') {
    const roleByField = { artistPortrait: 'artist', crewLogo: 'crew', labelLogo: 'label' } as const;
    const requiredRole = roleByField[field as keyof typeof roleByField];
    if (requiredRole && !record.roles.includes(requiredRole)) throw new Error(`Field ${field} requires the ${requiredRole} role`);
  }

  return definition.schema.parse(value);
}

export function missingFields(recordInput: CatalogRecord): string[] {
  const record = CatalogRecordSchema.parse(recordInput);
  if (record.kind === 'mix') return missingMixFields(record);
  if (record.kind === 'entity') return missingEntityFields(record);
  return missingEventFields(record);
}

function missingMixFields(record: MixRecord): string[] {
  const missing: string[] = [];
  if (!record.title.trim()) missing.push('title');
  if (!record.durationMs) missing.push('durationMs');
  if (!record.recordingDate) missing.push('recordingDate');
  if (!record.people.some((person) => person.role === 'artist')) missing.push('artists');
  if (!record.people.some((person) => person.role === 'crew')) missing.push('crews');
  if (!record.people.some((person) => person.role === 'label')) missing.push('labels');
  if (!record.eventIds.length) missing.push('events');
  if (!record.assets.some((asset) => asset.role === 'mix-cover')) missing.push('cover');
  if (!record.description) missing.push('description');
  if (!record.genres.length) missing.push('genres');
  if (!record.styles.length) missing.push('styles');
  return missing;
}

function missingEntityFields(record: EntityRecord): string[] {
  const missing: string[] = [];
  if (!record.profile?.trim()) missing.push('profile');
  if (!record.country) missing.push('country');
  for (const role of record.roles) {
    const assetRole = role === 'artist' ? 'artist-portrait' : `${role}-logo`;
    if (!record.assets.some((asset) => asset.role === assetRole)) {
      missing.push(role === 'artist' ? 'artistPortrait' : role === 'crew' ? 'crewLogo' : 'labelLogo');
    }
  }
  return missing;
}

function missingEventFields(record: EventRecord): string[] {
  const missing: string[] = [];
  if (!record.startDate) missing.push('startDate');
  if (!record.venue) missing.push('venue');
  if (!record.locality) missing.push('locality');
  if (!record.country) missing.push('country');
  if (!record.assets.some((asset) => asset.role === 'event-flyer')) missing.push('flyer');
  return missing;
}