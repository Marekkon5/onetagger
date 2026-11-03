import type { ApiMixCandidate } from '../services/api';
import { ImportCandidateSchema, CatalogRecordSchema, missingFields, type CatalogRecord, type EnrichmentReport, type RecordId } from '@syco23/catalog-domain';
import { humanValue } from './presentation';

export interface MetadataRow { field: string; label: string; value: unknown }
export const recordName = (record: CatalogRecord) => record.kind === 'mix' ? record.title : record.kind === 'entity' ? record.displayName : record.name;
export const recordLink = (record: CatalogRecord) => `/${record.kind === 'mix' ? 'mix' : record.kind === 'entity' ? 'entity' : 'event'}/${encodeURIComponent(record.id)}`;
export const formatValue = humanValue;
export function metadataRows(record: CatalogRecord): MetadataRow[] {
  const rows: MetadataRow[] = [];
  const add = (field: string, label: string, value: unknown) => rows.push({ field, label, value });
  if (record.kind === 'mix') {
    add('durationMs', 'Duration', record.durationMs ? `${(record.durationMs / 60000).toFixed(1)} minutes` : undefined);
    add('recordingDate', 'Recording date', record.recordingDate);
    add('genres', 'Genres', record.genres); add('styles', 'Styles', record.styles); add('description', 'Description', record.description);
    add('playbackUrls', 'Playback links', record.playbackUrls);
  } else if (record.kind === 'entity') {
    add('aliases', 'Aliases', record.aliases); add('country', 'Country', record.country); add('profile', 'Profile', record.profile);
    if (record.roles.includes('artist')) {
      add('artist.realName', 'Real name', record.artist?.realName); add('artist.profile', 'Artist profile', record.artist?.profile);
      add('artist.country', 'Artist country', record.artist?.country); add('artist.groupIds', 'Groups', record.artist?.groupIds); add('artist.memberIds', 'Artist members', record.artist?.memberIds);
    }
    if (record.roles.includes('crew')) {
      add('crew.profile', 'Crew profile', record.crew?.profile); add('crew.country', 'Crew country', record.crew?.country);
      add('crew.websiteUrls', 'Crew websites', record.crew?.websiteUrls); add('crew.memberIds', 'Crew members', record.crew?.memberIds);
    }
    if (record.roles.includes('label')) {
      add('label.profile', 'Label profile', record.label?.profile); add('label.country', 'Label country', record.label?.country);
      add('label.websiteUrls', 'Label websites', record.label?.websiteUrls); add('label.parentId', 'Parent label', record.label?.parentId); add('label.subLabelIds', 'Sub-labels', record.label?.subLabelIds);
    }
  } else {
    add('startDate', 'Start date', record.startDate); add('endDate', 'End date', record.endDate); add('venue', 'Venue', record.venue);
    add('locality', 'Locality', record.locality); add('country', 'Country', record.country); add('sourceUrls', 'Event sources', record.sourceUrls);
  }
  return rows;
}
export function relationshipIds(record: CatalogRecord): RecordId[] {
  if (record.kind === 'mix') return [...new Set([...record.people.map((p) => p.entityId), ...record.eventIds])];
  if (record.kind === 'event') return record.mixIds;
  return [...new Set([...(record.artist?.groupIds ?? []), ...(record.artist?.memberIds ?? []), ...(record.crew?.memberIds ?? []), ...(record.label?.subLabelIds ?? []), ...(record.label?.parentId ? [record.label.parentId] : [])])];
}

export type EditorKind = 'text' | 'longtext' | 'number' | 'list' | 'date' | 'roles' | 'assets' | 'people';
export interface EditorField { key: string; label: string; kind: EditorKind; required?: boolean }
export function editorFields(record: CatalogRecord): EditorField[] {
  const field = (key: string, label: string, kind: EditorKind = 'text', required = false): EditorField => ({ key, label, kind, required });
  if (record.kind === 'mix') return [field('title', 'Title', 'text', true), field('description', 'Description', 'longtext'), field('durationMs', 'Duration in milliseconds', 'number'), field('recordingDate', 'Recording date (YYYY, YYYY-MM or YYYY-MM-DD)', 'date'), field('genres', 'Genres (one per line)', 'list'), field('styles', 'Styles (one per line)', 'list'), field('people', 'Linked performers, crews and labels', 'people'), field('eventIds', 'Event record IDs (one per line)', 'list'), field('playbackUrls', 'Playback URLs (one per line)', 'list'), field('assets', 'Mix cover and stored waveform', 'assets')];
  if (record.kind === 'event') return [field('name', 'Event name', 'text', true), field('startDate', 'Start date (YYYY, YYYY-MM or YYYY-MM-DD)', 'date'), field('endDate', 'End date (YYYY, YYYY-MM or YYYY-MM-DD)', 'date'), field('venue', 'Venue'), field('locality', 'Locality'), field('country', 'Country ISO code'), field('sourceUrls', 'Source URLs (one per line)', 'list'), field('mixIds', 'Mix record IDs (one per line)', 'list'), field('assets', 'Event flyers', 'assets')];
  const result = [field('displayName', 'Display name', 'text', true), field('roles', 'Confirmed roles', 'roles', true), field('aliases', 'Aliases (one per line)', 'list'), field('profile', 'Profile', 'longtext'), field('country', 'Country ISO code')];
  for (const role of record.roles) {
    result.push(field(`${role}.profile`, `${role} profile`, 'longtext'), field(`${role}.country`, `${role} country ISO code`));
    if (role === 'artist') result.push(field('artist.realName', 'Real name'), field('artist.groupIds', 'Group record IDs (one per line)', 'list'), field('artist.memberIds', 'Artist member IDs (one per line)', 'list'));
    if (role === 'crew') result.push(field('crew.websiteUrls', 'Crew website URLs (one per line)', 'list'), field('crew.memberIds', 'Crew member IDs (one per line)', 'list'));
    if (role === 'label') result.push(field('label.websiteUrls', 'Label website URLs (one per line)', 'list'), field('label.parentId', 'Parent label record ID'), field('label.subLabelIds', 'Sub-label record IDs (one per line)', 'list'));
  }
  result.push(field('assets', 'Portraits and logos', 'assets'));
  return result;
}
function valueAt(record: CatalogRecord, key: string): unknown {
  return key.split('.').reduce<unknown>((value, segment) => value && typeof value === 'object' ? (value as Record<string, unknown>)[segment] : undefined, record);
}
export function editorDraft(record: CatalogRecord): Record<string, any> {
  return Object.fromEntries(editorFields(record).map((field) => {
    const value = valueAt(record, field.key);
    if (field.kind === 'list') return [field.key, Array.isArray(value) ? value.join('\n') : ''];
    if (field.kind === 'date') return [field.key, value && typeof value === 'object' && 'value' in value ? value.value : ''];
    if (['roles', 'assets', 'people'].includes(field.kind)) return [field.key, structuredClone(value ?? [])];
    return [field.key, value ?? ''];
  }));
}
export function parseEditorPatch(record: CatalogRecord, draft: Record<string, unknown>): Record<string, unknown> {
  const patch: Record<string, unknown> = {};
  const definitions = editorFields(record);
  for (const [key, input] of Object.entries(draft)) {
    const definition = definitions.find((field) => field.key === key);
    let value = input;
    if (['artist', 'crew', 'label'].includes(key) && typeof input === 'string') value = JSON.parse(input);
    else if (definition?.kind === 'list' && typeof input === 'string') value = [...new Set(input.split(/\r?\n/).map((line) => line.trim()).filter(Boolean))];
    else if (definition?.kind === 'number') value = input === '' ? undefined : Number(input);
    else if (definition?.kind === 'date') {
      const date = String(input).trim(); value = date ? { value: date, precision: date.length === 4 ? 'year' : date.length === 7 ? 'month' : 'day' } : undefined;
    } else if (key === 'assets' && typeof input === 'string') value = JSON.parse(input);
    else if (typeof input === 'string') value = input.trim() || (definition?.required ? '' : undefined);
    if (key === 'country' || key.endsWith('.country')) value = typeof value === 'string' ? value.toUpperCase() : value;
    const [root, nested] = key.split('.');
    if (nested) {
      const details = patch[root] as Record<string, unknown> | undefined ?? { ...((record as unknown as Record<string, unknown>)[root] as Record<string, unknown> ?? {}) };
      if (value === undefined) delete details[nested]; else details[nested] = value;
      patch[root] = details;
    } else patch[root] = value;
  }
  if (record.kind === 'entity' && Array.isArray(patch.roles)) for (const role of ['artist', 'crew', 'label']) if (!patch.roles.includes(role) && (record as unknown as Record<string, unknown>)[role] !== undefined) patch[role] = undefined;
  const candidate = { ...record, ...patch };
  for (const [key, value] of Object.entries(candidate)) if (value === undefined) delete (candidate as Record<string, unknown>)[key];
  const validated = CatalogRecordSchema.parse(candidate);
  return Object.fromEntries(Object.keys(patch).map((key) => [key, (validated as unknown as Record<string, unknown>)[key] ?? null]));
}
export function enrichmentSummary(report: EnrichmentReport): string {
  const added = report.applied ? `${report.applied} ${report.applied === 1 ? 'field' : 'fields'} added` : 'No fields added';
  const corroborated = `${report.corroborated} ${report.corroborated === 1 ? 'source agreed' : 'sources agreed'}`;
  const reviewed = `${report.reviewed} sent to Review`;
  const errors = `${report.errors.length} ${report.errors.length === 1 ? 'provider unavailable' : 'providers unavailable'}`;
  const gaps = report.missingFields.length ? `${report.missingFields.length} ${report.missingFields.length === 1 ? 'field' : 'fields'} still missing` : 'No fields currently missing';
  return `${added} · ${corroborated} · ${reviewed} · ${errors} · ${gaps}`;
}
export function duplicatePreview(survivor: CatalogRecord, duplicate: CatalogRecord) {
  if (survivor.id === duplicate.id) throw new Error('Choose two different records');
  if (survivor.kind !== duplicate.kind) throw new Error('Duplicate records must have the same type');
  return { survivor, duplicate, revisions: [survivor.revision, duplicate.revision] as [number, number], survivorGaps: missingFields(survivor), duplicateGaps: missingFields(duplicate) };
}

export function toImportCandidate(candidate: ApiMixCandidate) {
  const resourceType = candidate.provider === 'youtube' ? 'video' : candidate.provider === 'mixcloud' ? 'cloudcast' : ['soundcloud', 'hearthis'].includes(candidate.provider) ? 'track' : candidate.provider === 'archiveorg' ? 'item' : 'recording';
  return ImportCandidateSchema.parse({
    provider: candidate.provider, title: candidate.title, artists: candidate.artists ?? [], crews: candidate.crews ?? [],
    durationMs: candidate.durationMs, recordedAt: candidate.recordedAt, description: candidate.description,
    genres: candidate.genres ?? [], artwork: candidate.artwork ?? [], confidence: candidate.confidence, reasons: candidate.reasons ?? [],
    fieldEvidence: candidate.fieldEvidence, uploadedAt: candidate.uploadedAt, uploader: candidate.uploader,
    source: { provider: candidate.provider, resourceType, externalId: candidate.source.externalId || candidate.source.url, url: candidate.source.url }
  });
}
