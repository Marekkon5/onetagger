import assert from 'node:assert/strict';
import test from 'node:test';
import {
  CatalogRecordSchema,
  FieldClaimSchema,
  normalizeName,
  missingFields,
  type MixRecord
} from './index.js';

const timestamp = '2026-10-03T12:00:00.000Z';

function mix(overrides: Partial<MixRecord> = {}): MixRecord {
  return {
    kind: 'mix',
    id: 'mix_01J9CATALOGUE00000000000000',
    createdAt: timestamp,
    updatedAt: timestamp,
    revision: 1,
    verification: 'source-confirmed',
    reviewState: 'ready',
    title: 'Live Mackitek Koalisson III',
    people: [],
    eventIds: [],
    genres: [],
    styles: [],
    assets: [],
    sources: [],
    ...overrides
  };
}

test('name keys normalize Unicode and whitespace without erasing identity words', () => {
  assert.equal(normalizeName('  DJ   Cafe\u0301  '), normalizeName('dj café'));
  assert.equal(normalizeName('Live Mackitek'), 'live mackitek');
  assert.equal(normalizeName('DJ Koalisson'), 'dj koalisson');
});

test('canonical records preserve display spelling and reject unknown fields', () => {
  const parsed = CatalogRecordSchema.parse(mix({ title: '  DJ Café  ' }));
  assert.equal(parsed.kind, 'mix');
  assert.equal(parsed.title, '  DJ Café  ');
  assert.equal(CatalogRecordSchema.safeParse({ ...mix(), mystery: true }).success, false);
});

test('recording dates retain precision and cannot be confused with upload dates', () => {
  assert.equal(CatalogRecordSchema.safeParse(mix({ recordingDate: { value: '2024-13', precision: 'month' } })).success, false);
  const claim = FieldClaimSchema.parse({
    targetRecordId: 'mix_01J9CATALOGUE00000000000000',
    field: 'recordingDate',
    value: { value: '2024', precision: 'year' },
    provider: { provider: 'youtube', resourceType: 'video', externalId: 'abc123' },
    sourceUrl: 'https://youtube.com/watch?v=abc123',
    observedAt: timestamp,
    evidence: 'direct',
    matchExplanation: 'Matched recording title and duration'
  });
  assert.equal(claim.field, 'recordingDate');
  assert.equal('uploadDate' in claim, false);
});

test('a portrait cannot satisfy the mix-cover completeness field', () => {
  const record = mix({
    assets: [{ role: 'artist-portrait', url: 'https://example.org/portrait.jpg', source: 'discogs' }]
  });
  assert.ok(missingFields(record).includes('cover'));
  assert.equal(CatalogRecordSchema.safeParse(mix({ durationMs: -1 })).success, false);
});

test('entity countries must be recognized region codes, not arbitrary two-letter strings', () => {
  const entity = {
    kind: 'entity',
    id: 'entity_01J9CATALOGUE00000000',
    createdAt: timestamp,
    updatedAt: timestamp,
    revision: 1,
    verification: 'source-confirmed',
    reviewState: 'ready',
    displayName: 'Mackitek',
    roles: ['crew'],
    aliases: [],
    assets: [],
    providerRefs: [],
    country: 'XX'
  };
  assert.equal(CatalogRecordSchema.safeParse(entity).success, false);
});