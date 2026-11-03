import { z } from 'zod';
import { ArtistDetailsSchema, CrewDetailsSchema, LabelDetailsSchema, CatalogRecordSchema, CountryCodeSchema, RecordingDateSchema, RecordIdSchema, HttpUrlSchema, MediaAssetSchema, EntityRoleSchema, ProviderSourceSchema, ProviderRefSchema, type CatalogRecord } from './schemas.js';
import { normalizeName } from './normalize.js';

const textValue = z.string().min(1).max(500).refine((value) => !!value.trim());
const normalizedSet = <T>(values: T[], key: (value: T) => string) => values.filter((value, index) => values.findIndex((item) => key(item) === key(value)) === index);
const textList = z.array(textValue).max(100).transform((values) => normalizedSet(values, normalizeName));
const ids = z.array(RecordIdSchema).max(100).transform((values) => [...new Set(values)]);
const urls = z.array(HttpUrlSchema).max(100).transform((values) => [...new Set(values)]);
const field = (targets: readonly string[], schema: z.ZodType, roles?: readonly string[]) => ({ targets, schema, roles });
const mix = ['mix']; const entity = ['entity']; const event = ['event'];
export const FieldDefinitions = {
  title: field(mix,textValue), description: field(mix,z.string().max(10000)), durationMs: field(mix,z.number().finite().int().positive().max(86400000)), recordingDate: field(mix,RecordingDateSchema),
  genres: field(mix,textList), styles: field(mix,textList), people: field(mix,z.array(z.object({ entityId: RecordIdSchema, role: EntityRoleSchema }).strict()).max(100)), eventIds: field(mix,ids), playbackUrls: field(mix,urls), sources: field(mix,z.array(ProviderSourceSchema).max(100)),
  cover: field(mix,z.object({ role: z.literal('mix-cover'), url: HttpUrlSchema }).strict()),
  artistPortrait: field(entity,z.object({ role: z.literal('artist-portrait'),url: HttpUrlSchema }).strict(),['artist']), crewLogo: field(entity,z.object({role:z.literal('crew-logo'),url:HttpUrlSchema}).strict(),['crew']), labelLogo: field(entity,z.object({role:z.literal('label-logo'),url:HttpUrlSchema}).strict(),['label']),
  providerRefs: field(entity,z.array(ProviderRefSchema).max(100)), displayName: field(entity,textValue), roles: field(entity,z.array(EntityRoleSchema).min(1).max(3)), aliases: field(entity,textList), profile: field(entity,z.string().max(20000)), country: field(['entity','event'],CountryCodeSchema),
  artist: field(entity,ArtistDetailsSchema,['artist']), crew: field(entity,CrewDetailsSchema,['crew']), label: field(entity,LabelDetailsSchema,['label']), realName: field(entity,textValue,['artist']), groupIds: field(entity,ids,['artist']), memberIds: field(entity,ids,['artist','crew']), parentId: field(entity,RecordIdSchema,['label']), subLabelIds: field(entity,ids,['label']), websiteUrls: field(entity,urls,['crew','label']),
  name: field(event,textValue), eventDate: field(event,RecordingDateSchema), startDate: field(event,RecordingDateSchema), endDate: field(event,RecordingDateSchema), venue: field(event,textValue), locality: field(event,textValue), sourceUrls: field(event,urls), mixIds: field(event,ids), flyer: field(event,z.object({role:z.literal('event-flyer'),url:HttpUrlSchema}).strict()),
  possibleDuplicate: field(['mix','entity','event'],z.object({recordId:RecordIdSchema,revision:z.number().int().positive()}).strict()),
  assets: field(['mix','entity','event'],z.array(MediaAssetSchema).max(100))
} as const;
export type FieldName = keyof typeof FieldDefinitions;
export function validateField(record: CatalogRecord, name: string, value: unknown): unknown {
  const definition = FieldDefinitions[name as FieldName];
  if (!definition) throw new Error(`Unsupported catalog field: ${name}`);
  if (!definition.targets.includes(record.kind)) throw new Error(`Field ${name} does not apply to ${record.kind} records`);
  if (definition.roles && (record.kind !== 'entity' || !definition.roles.some((role) => record.roles.includes(role as 'artist'|'crew'|'label')))) throw new Error(`Field ${name} requires the ${definition.roles.join(' or ')} role`);
  if(value===null && ['description','durationMs','recordingDate','profile','country','artist','crew','label','realName','parentId','startDate','endDate','venue','locality','playbackUrls','memberIds','groupIds','subLabelIds','websiteUrls'].includes(name)) return null;
  return definition.schema.parse(value);
}
const assetRoles: Record<string,string> = {cover:'mix-cover',artistPortrait:'artist-portrait',crewLogo:'crew-logo',labelLogo:'label-logo',flyer:'event-flyer'};
function detailRole(record: CatalogRecord, name: string): 'artist'|'crew'|'label'|undefined {
  if (record.kind !== 'entity') return;
  if (['realName','groupIds'].includes(name)) return 'artist';
  if (['parentId','subLabelIds'].includes(name)) return 'label';
  if (name === 'memberIds') return record.roles.includes('crew') ? 'crew' : 'artist';
  if (name === 'websiteUrls') return record.roles.includes('label') ? 'label' : 'crew';
}
export function getFieldValue(record: CatalogRecord, name: string): unknown {
  if (assetRoles[name]) { const asset=record.assets.find((item)=>item.role===assetRoles[name]); return asset ? {role:asset.role,url:asset.url}:undefined; }
  if (name==='eventDate' && record.kind==='event') return record.startDate;
  const role=detailRole(record,name); const value=role && record.kind==='entity' ? (record[role] as Record<string,unknown>|undefined)?.[name] : (record as unknown as Record<string,unknown>)[name];
  return (Array.isArray(value) && value.length===0) || (typeof value==='string' && !value.trim()) ? undefined : value;
}
export function setFieldValue(record: CatalogRecord, name: string, input: unknown, source: 'local'|'curator'|'freeteknomusic'|'soundcloud'|'archiveorg'|'discogs'|'youtube'|'hearthis'|'mixcloud' = 'curator'): CatalogRecord {
  if(name==='possibleDuplicate')throw new Error('Possible duplicate acceptance requires the duplicate merge action');
  const value=validateField(record,name,input);
  if(value===null) {
    const role=detailRole(record,name);
    if(role&&record.kind==='entity'){const detail={...record[role]} as Record<string,unknown>;delete detail[name];return CatalogRecordSchema.parse({...record,[role]:detail});}
    const next={...record} as unknown as Record<string,unknown>;delete next[name];return CatalogRecordSchema.parse(next);
  }
  if (assetRoles[name]) return CatalogRecordSchema.parse({...record,assets:[...record.assets.filter((asset)=>asset.role!==assetRoles[name]),{...(value as object),source}]});
  if (name==='eventDate' && record.kind==='event') return CatalogRecordSchema.parse({...record,startDate:value});
  const role=detailRole(record,name);
  return CatalogRecordSchema.parse(role && record.kind==='entity' ? {...record,[role]:{...record[role],[name]:value}} : {...record,[name]:value});
}
export function missingFields(input: CatalogRecord): string[] {
  const record=CatalogRecordSchema.parse(input);
  const fields=record.kind==='mix' ? ['durationMs','recordingDate','cover','description','genres','styles'] : record.kind==='entity' ? ['profile','country',...record.roles.map((role)=>role==='artist'?'artistPortrait':role==='crew'?'crewLogo':'labelLogo')] : ['startDate','venue','locality','country','flyer'];
  const missing=fields.filter((name)=>getFieldValue(record,name)===undefined);
  if (record.kind==='mix') { for (const role of ['artist','crew','label'] as const) if(!record.people.some((person)=>person.role===role)) missing.push(`${role}s`); if(!record.eventIds.length) missing.push('events'); }
  return missing;
}
