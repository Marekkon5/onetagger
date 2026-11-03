import { describe, expect, it } from 'vitest';
import type { EntityRecord, EventRecord, MixRecord } from '@syco23/catalog-domain';

const base = { id: 'record_0123456789abcdef', createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:00.000Z', revision: 1, verification: 'curator-confirmed' as const, reviewState: 'ready' as const };
const entity: EntityRecord = { ...base, kind: 'entity', displayName: 'Artist', roles: ['artist', 'label'], aliases: [], assets: [], providerRefs: [], artist: { realName: 'Real artist', memberIds: ['entity_member012345'] }, label: { parentId: 'entity_parent012345', websiteUrls: ['https://example.org/'] } };
const event: EventRecord = { ...base, kind: 'event', name: 'Event', startDate: { value: '2024-10', precision: 'month' }, endDate: { value: '2024-11', precision: 'month' }, sourceUrls: ['https://example.org/event'], assets: [], mixIds: [] };
const mix: MixRecord = { ...base, kind: 'mix', title: 'Mix', people: [], eventIds: [event.id], genres: [], styles: [], assets: [], sources: [], recordingDate: { value: '2023', precision: 'year' } };
async function helpers() { return import('./views').catch(() => undefined) as Promise<any>; }

describe('catalog view projections', () => {
  it('projects real name, label website and relationships from type-specific entity fields', async () => {
    const module = await helpers();
    expect(module?.metadataRows, 'typed metadata projection is available').toBeTypeOf('function');
    const rows = module.metadataRows(entity);
    expect(rows).toEqual(expect.arrayContaining([expect.objectContaining({ field: 'artist.realName', value: 'Real artist' }), expect.objectContaining({ field: 'label.websiteUrls', value: ['https://example.org/'] })]));
    expect(module.relationshipIds(entity)).toEqual(['entity_member012345', 'entity_parent012345']);
  });
  it('preserves event start/end precision without replacing a mix recording date', async () => {
    const module = await helpers();
    expect(module?.metadataRows).toBeTypeOf('function');
    expect(module.metadataRows(event)).toEqual(expect.arrayContaining([expect.objectContaining({ field: 'endDate', value: event.endDate })]));
    expect(module.metadataRows(mix)).toEqual(expect.arrayContaining([expect.objectContaining({ field: 'recordingDate', value: mix.recordingDate })]));
  });
  it('validates typed editor input and retains assets and relationship IDs in the committed patch', async () => {
    const module = await helpers();
    expect(module?.parseEditorPatch).toBeTypeOf('function');
    expect(module.parseEditorPatch(entity, { displayName: 'Correct name', country: 'DE', aliases: 'Alias one\nAlias two', artist: JSON.stringify(entity.artist), label: JSON.stringify(entity.label), assets: '[]', roles: ['artist', 'label'] })).toMatchObject({ aliases: ['Alias one', 'Alias two'], artist: entity.artist, label: entity.label, assets: [] });
    expect(() => module.parseEditorPatch(entity, { country: 'invalid' })).toThrow();
  });
  it('describes unavailable providers and remaining unsupported gaps independently from applied claims', async () => {
    const module = await helpers();
    expect(module?.enrichmentSummary).toBeTypeOf('function');
    const text = module.enrichmentSummary({ state: 'completed', attemptedProviders: ['youtube'], applied: 0, corroborated: 0, reviewed: 0, errors: [{ provider: 'youtube', message: 'Provider unavailable' }], missingFields: ['cover'] });
    expect(text).toContain('No fields added'); expect(text).toContain('1 field'); expect(text).toContain('unavailable'); expect(text).not.toContain('complete');
  });
});

it('does not emit absent artist or label details while editing a crew', async () => {
  const module = await helpers();
  const crew: EntityRecord = { ...entity, roles: ['crew'], artist: undefined, label: undefined, crew: { profile: 'Original crew' } };
  const patch = module.parseEditorPatch(crew, { displayName: 'Artist', roles: ['crew'], profile: 'Curated crew', country: 'DE', 'crew.profile': 'Original crew', assets: [], aliases: '' });
  expect(patch).not.toHaveProperty('artist'); expect(patch).not.toHaveProperty('label');
});

it('retains parsed recording evidence and distinct uploader/upload facts in an import payload', async () => {
  const module = await helpers();
  expect(module?.toImportCandidate).toBeTypeOf('function');
  const result = module.toImportCandidate({ provider: 'youtube', title: 'Title', artists: [], crews: [], recordedAt: '2023', uploadedAt: '2026-01-01', uploader: 'Channel owner', fieldEvidence: { recordingDate: 'parsed' }, artwork: [], genres: [], source: { provider: 'youtube', url: 'https://youtube.com/watch?v=video1', externalId: 'video1' }, confidence: .8, reasons: [] });
  expect(result.fieldEvidence).toEqual({ recordingDate: 'parsed' }); expect(result.uploadedAt).toBe('2026-01-01'); expect(result.uploader).toBe('Channel owner');
});
